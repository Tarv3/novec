pub mod mapped;
pub mod oom;
pub mod persistant;
pub mod novec;
pub mod loader;

pub use crate::novec::*;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum KeyIdx<Q> {
    Key(Q),
    Index(usize),
    Both { key: Q, index: usize },
}

impl<Q> KeyIdx<Q> {
    pub fn new(key: Option<Q>, index: Option<usize>) -> Option<KeyIdx<Q>> {
        match (key, index) {
            (Some(key), Some(index)) => Some(KeyIdx::Both { key, index }),
            (Some(key), None) => Some(KeyIdx::Key(key)),
            (None, Some(index)) => Some(KeyIdx::Index(index)),
            (None, None) => None
        }
    }

    pub fn has_key(&self) -> bool {
        match self {
            KeyIdx::Key(_) | KeyIdx::Both { .. } => true,
            _ => false
        }
    }

    pub fn has_index(&self) -> bool {
        match self {
            KeyIdx::Index(_) | KeyIdx::Both { .. } => true,
            _ => false
        }
    } 

    pub fn key(&self) -> Option<&Q> {
        match self {
            KeyIdx::Both { key, .. } => Some(key),
            KeyIdx::Key(key) => Some(key),
            _ => None,
        }
    }

    pub fn index(&self) -> Option<usize> {
        match self {
            KeyIdx::Both { index, .. } => Some(*index),
            KeyIdx::Index(index) => Some(*index),
            _ => None,
        }
    }
}