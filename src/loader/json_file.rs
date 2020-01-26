use super::*;
use cbc::*;
use serde::de::DeserializeOwned;
use std::{
    collections::HashMap,
    error::Error,
    fmt::{self, Display, Formatter},
    hash::Hash,
    io::BufRead,
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

pub struct JsonFile<K: Hash, T> {
    mapping: HashMap<K, PathBuf>,
    receiver: Receiver<(K, OneTimeLock<T>)>,
}

impl<K: Hash + Clone + Eq, T> JsonFile<K, T> {
    pub fn new(receiver: Receiver<(K, OneTimeLock<T>)>) -> Self {
        Self {
            mapping: HashMap::new(),
            receiver,
        }
    }

    pub fn from_file(
        receiver: Receiver<(K, OneTimeLock<T>)>,
        path: impl AsRef<Path>,
    ) -> Result<Self, Box<dyn Error>>
    where
        K: DeserializeOwned,
    {
        let (_, mappings) = load_mappings_from_file(path)?;

        Ok(Self::from_mappings(receiver, mappings.into_iter()))
    }

    pub fn from_mappings(
        receiver: Receiver<(K, OneTimeLock<T>)>,
        mappings: impl Iterator<Item = (K, PathBuf)>,
    ) -> Self {
        let mut mapping = HashMap::new();

        for (key, path) in mappings {
            mapping.insert(key, path);
        }

        Self { mapping, receiver }
    }

    pub fn receive<U>(&self, f: impl Fn(U) -> T) 
    where 
        U: DeserializeOwned,
    {
        for (key, mut into) in self.receiver.iter() {
            let path = match self.mapping.get(&key) {
                Some(value) => value.as_path(),
                None => continue
            };

            let file = match std::fs::File::open(path) {
                Ok(file) => file,
                Err(e) => {
                    dbg!(e);
                    continue;
                }
            };

            let reader = std::io::BufReader::new(file);
            let loaded = match serde_json::from_reader(reader) {
                Ok(value) => f(value),
                Err(e) => {
                    dbg!(e);
                    continue;
                }
            };

            into.write(loaded);
            into.unlock();
        }
    }
}
