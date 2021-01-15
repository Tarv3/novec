use super::*;
use std::{
    any::TypeId,
    collections::HashMap,
    error::Error,
    fmt::{self, Debug, Display, Formatter},
    fs::File,
    hash::Hash,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    str::FromStr,
};

#[derive(Copy, Clone, Debug)]
pub enum MappingError {
    MissingMapping(usize),
    ParseError(usize),
}

impl Display for MappingError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            MappingError::MissingMapping(line) => write!(f, "Missing file mapping for key line {}", line),
            MappingError::ParseError(line) => write!(f, "Failed to parse key line {}", line),
        }
    }
}

impl Error for MappingError {}

fn load_mappings_from_file<K: FromStr>(
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
        None => return Err(Box::new(MappingError::MissingMapping(0))),
    };

    let mut mappings = vec![];

    for (i, line) in lines.enumerate() {
        let line = line?;

        let mut split = line.split("=>");
        let key = match split.next() {
            Some(key) => key
                .trim()
                .parse()
                .or_else(|_| Err(MappingError::ParseError(i)))?,
            None => return Err(Box::new(MappingError::MissingMapping(i))),
        };

        let path = match split.next() {
            Some(path) => {
                let mut pbuf = parent.clone();
                pbuf.push(path.trim());
                pbuf
            }
            None => return Err(Box::new(MappingError::MissingMapping(i))),
        };

        mappings.push((key, path));
    }

    Ok((parent, mappings))
}

pub struct MappedObject<'a, K> {
    pub type_id: TypeId,
    pub key: K,
    pub path: &'a Path,
    pub reader: BufReader<File>,
}

pub enum MapError {
    MissingMapping,
    FileError(PathBuf, std::io::Error),
}

pub struct FileMapper<K: Hash> {
    parent: PathBuf,
    mapping: HashMap<K, PathBuf>,
    receiver: GenericReceiver<K>,
    shutdown: Option<Receiver<()>>,
}

impl<K: Hash + Clone + Eq> FileMapper<K> {
    pub fn new(receiver: GenericReceiver<K>, shutdown: Option<Receiver<()>>) -> Self {
        Self {
            parent: PathBuf::new(),
            mapping: HashMap::new(),
            receiver,
            shutdown,
        }
    }

    pub fn from_file(
        receiver: GenericReceiver<K>,
        shutdown: Option<Receiver<()>>,
        path: impl AsRef<Path>,
    ) -> Result<Self, Box<dyn Error>>
    where
        K: FromStr,
    {
        let (parent, mappings) = load_mappings_from_file(path)?;

        Ok(Self::from_mappings(
            receiver,
            shutdown,
            parent,
            mappings.into_iter(),
        ))
    }

    pub fn from_mappings(
        receiver: GenericReceiver<K>,
        shutdown: Option<Receiver<()>>,
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
            shutdown,
        }
    }

    pub fn receive_non_blocking(
        &self,
        mut success: impl FnMut(MappedObject<K>) -> GenericResult,
        mut fail: impl FnMut(K, MapError),
    ) -> Result<(), RecvError> {
        if let Some(r) = self.shutdown.as_ref() {
            match r.try_recv() {
                Ok(_) => return Ok(()),
                Err(TryRecvError::Empty) => {}
                Err(_) => return Err(RecvError),
            }
        }

        for (key, into) in self.receiver.try_iter() {
            let path = match self.mapping.get(&key) {
                Some(value) => value,
                None => {
                    fail(key, MapError::MissingMapping);
                    return Ok(());
                }
            };

            let file = match std::fs::File::open(&path) {
                Ok(file) => file,
                Err(e) => {
                    fail(key, MapError::FileError(path.clone(), e));
                    return Ok(());
                }
            };

            let reader = std::io::BufReader::new(file);

            let mapped = MappedObject {
                type_id: into.meta_data,
                key,
                path: path.as_path(),
                reader,
            };

            if let Err(_) = into.send(success(mapped)) {
                // @ErrorHandling
                dbg!("Load send error");
            }
        }

        Ok(())
    }

    pub fn receive(
        &self,
        mut success: impl FnMut(MappedObject<K>) -> GenericResult,
        mut fail: impl FnMut(K, MapError),
    ) -> Result<(), RecvError> {
        loop {
            select! {
                recv(self.shutdown.as_ref().unwrap_or(&cbc::never())) -> _ => break,
                recv(self.receiver) -> msg => match msg {
                    Ok((key, into)) => {
                        let path = match self.mapping.get(&key) {
                            Some(value) => value,
                            None => {
                                fail(key, MapError::MissingMapping);
                                continue;
                            },
                        };

                        let file = match std::fs::File::open(&path) {
                            Ok(file) => file,
                            Err(e) => {
                                fail(key, MapError::FileError(path.clone(), e));
                                continue;
                            }
                        };

                        let reader = std::io::BufReader::new(file);

                        let mapped = MappedObject {
                            type_id: into.meta_data,
                            key,
                            path: path.as_path(),
                            reader,
                        };

                        if let Err(_) = into.send(success(mapped)) {
                            // @ErrorHandling
                        }
                    },
                    Err(e) => return Err(e)
                }
            }
        }

        Ok(())
    }
}
