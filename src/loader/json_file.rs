use std::{
    collections::HashMap,
    error::Error,
    fmt::{self, Display, Formatter},
    hash::Hash,
    path::{PathBuf, Path},
    borrow::Borrow,
    io::BufRead,
};
use serde::{Serialize, Deserialize, de::DeserializeOwned};
use super::*;

#[derive(Copy, Clone, Debug)]
pub struct MissingMapping;

impl Display for MissingMapping {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Missing file mapping for key")
    }
}

impl Error for MissingMapping {}

fn load_mappings_from_file<K: DeserializeOwned>(
    path: impl AsRef<Path>
) -> Result<(PathBuf, Vec<(K, PathBuf)>), Box<dyn Error>> {
    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    let mut lines = reader.lines();
    
    let parent = match lines.next() {
        Some(parent) => {
            let mut pbuf = PathBuf::new();
            pbuf.push(parent?);
            pbuf
        },
        None => return Err(Box::new(MissingMapping)),
    };

    let mut mappings = vec![];

    for line in lines {
        let line = line?;

        let mut split = line.split("=>");
        let key = match split.next() {
            Some(key) => serde_json::from_str(key)?,
            None => return Err(Box::new(MissingMapping))
        };

        let path = match split.next() {
            Some(path) => {
                let mut pbuf = parent.clone();
                pbuf.push(path.trim());
                pbuf
            },
            None => return Err(Box::new(MissingMapping)),
        };

        mappings.push((key, path));
    }

    Ok((parent, mappings))
}

pub struct JsonFile<K: Hash, T> {
    mapping: HashMap<K, PathBuf>,
    loaded: Vec<(K, T)>,
}

impl<K: Hash + Clone + Eq, T> JsonFile<K, T> {
    pub fn new() -> Self {
        Self {
            mapping: HashMap::new(),
            loaded: vec![]
        }
    }

    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, Box<dyn Error>>
    where
        K: DeserializeOwned
    {
        let (_, mappings) = load_mappings_from_file(path)?;

        Self::from_mappings(mappings.into_iter())
    }

    pub fn from_mappings(mappings: impl Iterator<Item = (K, PathBuf)>) -> Self {
        let mut mapping = HashMap::new();

        for (key, path) in mappings {
            mapping.insert(key, path);
        }

        Self {
            mapping,
            loaded: vec![]
        }
    }

    pub fn load<Q>(&self, key: Q) -> Result<T, Box<dyn Error>> 
    where
        T: DeserializeOwned,
        K: Borrow<Q>,
        Q: Hash + Eq
    {
        let path = match self.mapping.get(key) {
            Some(path) => path,
            None => return Err(Box::new(MissingMapping)),
        };

        let file = std::fs::File::open(path)?;
        let reader = std::io::BufReader::new(file);

        let data = serde_json::from_reader(reader)?;

        Ok(data)

    }
}