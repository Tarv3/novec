use super::*;
use serde::de::DeserializeOwned;
use std::{
    any::TypeId,
    collections::HashMap,
    error::Error,
    fmt::{self, Display, Formatter},
    fs::File,
    hash::Hash,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

#[derive(Copy, Clone, Debug)]
pub struct MissingMapping;

impl Display for MissingMapping {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Missing file mapping for key")
    }
}

impl Error for MissingMapping {}

fn load_mappings_from_file<K: DeserializeOwned>(
    path: impl AsRef<Path>,
) -> Result<(PathBuf, Vec<(K, PathBuf)>), Box<dyn Error>> {
    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    let mut lines = reader.lines();

    let parent = match lines.next() {
        Some(parent) => {
            let mut pbuf = PathBuf::new();
            pbuf.push(parent?);
            pbuf
        }
        None => return Err(Box::new(MissingMapping)),
    };

    let mut mappings = vec![];

    for line in lines {
        let line = line?;

        let mut split = line.split("=>");
        let key = match split.next() {
            Some(key) => serde_json::from_str(key)?,
            None => return Err(Box::new(MissingMapping)),
        };

        let path = match split.next() {
            Some(path) => {
                let mut pbuf = parent.clone();
                pbuf.push(path.trim());
                pbuf
            }
            None => return Err(Box::new(MissingMapping)),
        };

        mappings.push((key, path));
    }

    Ok((parent, mappings))
}

pub struct JsonFile<K: Hash> {
    parent: PathBuf,
    mapping: HashMap<K, PathBuf>,
    receiver: GenericReceiver<K>,
}

impl<K: Hash + Clone + Eq> JsonFile<K> {
    pub fn new(receiver: GenericReceiver<K>) -> Self {
        Self {
            parent: PathBuf::new(),
            mapping: HashMap::new(),
            receiver,
        }
    }

    pub fn from_file(
        receiver: GenericReceiver<K>,
        path: impl AsRef<Path>,
    ) -> Result<Self, Box<dyn Error>>
    where
        K: DeserializeOwned,
    {
        let (parent, mappings) = load_mappings_from_file(path)?;

        Ok(Self::from_mappings(receiver, parent, mappings.into_iter()))
    }

    pub fn from_mappings(
        receiver: GenericReceiver<K>,
        parent: PathBuf,
        mappings: impl Iterator<Item = (K, PathBuf)>,
    ) -> Self {
        let mut mapping = HashMap::new();

        for (key, path) in mappings {
            mapping.insert(key, path);
        }

        Self {
            parent,
            mapping,
            receiver,
        }
    }

    pub fn receive<E: Error>(&self, f: impl Fn(BufReader<File>, TypeId) -> Result<GenericItem, E>) {
        for (key, into) in self.receiver.iter() {
            let mut path = self.parent.clone();

            match self.mapping.get(&key) {
                Some(value) => path.push(value.as_path()),
                None => continue,
            }

            let file = match std::fs::File::open(path) {
                Ok(file) => file,
                Err(e) => {
                    dbg!(e);
                    continue;
                }
            };

            let reader = std::io::BufReader::new(file);

            let item = match f(reader, into.meta_data) {
                Ok(item) => item, 
                Err(e) => {
                    println!("Load error: {}", e);
                    continue
                }
            };
            into.send(item).expect("Failed to send loaded value");
        }
    }
}
