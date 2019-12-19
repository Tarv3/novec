use crate::*;
use std::{
    borrow::Borrow,
    collections::hash_map::{Entry as HashEntry, HashMap},
    hash::Hash,
};

pub struct Occupied<'a, K: 'a, T: 'a, I: 'a> {
    key: &'a K,
    index: &'a I,
    value: &'a mut T,
}

pub struct VacantEntry<'a, K: 'a, T: 'a, S: 'a>
where
    K: Hash + Eq,
    S: PersistantStorage<(K, T)>,
{
    key: K,
    storage: &'a mut MappedStorage<K, T, S>,
}

pub enum Entry<'a, K: 'a, T: 'a, S: 'a>
where
    K: Hash + Eq,
    S: PersistantStorage<(K, T)>,
{
    Occupied(Occupied<'a, K, T, S::Index>),
    Vacant(VacantEntry<'a, K, T, S>),
}

impl<'a, K: 'a, T: 'a, S: 'a> Entry<'a, K, T, S>
where
    K: Hash + Eq + Clone,
    S: PersistantStorage<(K, T)>,
    S::Index: Clone,
{
    pub fn key(&self) -> &K {
        match self {
            Entry::Occupied(occupied) => occupied.key,
            Entry::Vacant(vacant) => &vacant.key,
        }
    }

    pub fn or_insert(self, default: T) -> (&'a S::Index, &'a mut T) {
        match self {
            Self::Occupied(occupied) => (occupied.index, occupied.value),
            Self::Vacant(VacantEntry { key, storage }) => {
                let (index, value, _) = storage.insert_get(key, default);

                (index, value)
            }
        }
    }

    pub fn or_insert_with<F: FnOnce() -> T>(self, default: F) -> (&'a S::Index, &'a mut T) {
        self.or_insert(default())
    }

    pub fn and_modify<F: FnOnce(&mut T)>(mut self, f: F) -> Self {
        match &mut self {
            Entry::Occupied(Occupied { value, .. }) => f(value),
            _ => {}
        }

        self
    }

    pub fn or_default(self) -> (&'a S::Index, &'a mut T)
    where
        T: Default,
    {
        self.or_insert(Default::default())
    }
}

#[derive(Clone, Debug)]
pub struct MappedStorage<K, T, S>
where
    K: Hash + Eq,
    S: PersistantStorage<(K, T)>,
{
    indices: HashMap<K, S::Index>,
    storage: S,
}

impl<K, T, S> MappedStorage<K, T, S>
where
    K: Hash + Eq + Clone,
    S: PersistantStorage<(K, T)>,
    S::Index: Clone,
{
    pub fn new() -> Self
    where
        S: Default,
    {
        MappedStorage {
            indices: HashMap::new(),
            storage: S::default(),
        }
    }

    pub fn entry<Q, I>(&mut self, ki: KeyIdx<Q, I>) -> Option<Entry<K, T, S>>
    where
        K: Borrow<Q>,
        I: Borrow<S::Index>,
        Q: Hash + Eq + Into<K>,
    {
        match ki {
            KeyIdx::Index(index) | KeyIdx::Both { index, .. } => {
                let (key, value) = self.storage.get_mut(index.borrow())?;
                let index = self.indices.get((&*key).borrow()).unwrap();

                Some(Entry::Occupied(Occupied { key, index, value }))
            }
            KeyIdx::Key(key) => {
                let self_ptr = self as *mut Self;

                if let Some(index) = self.indices.get(&key) {
                    let (key, value) = self.storage.get_mut(index).unwrap();
                    Some(Entry::Occupied(Occupied { key, index, value }))
                } else {
                    // This is to avoid the borrow checker and is valid because nothing else will
                    // have a reference to self at this point in time 
                    unsafe {
                        Some(Entry::Vacant(VacantEntry {
                            key: key.into(),
                            storage: &mut *self_ptr,
                        }))
                    }
                }
            }
        }
    }

    pub fn get<Q, I>(&self, ki: &KeyIdx<Q, I>) -> Option<&T>
    where
        K: Borrow<Q>,
        I: Borrow<S::Index>,
        Q: Hash + Eq,
    {
        match ki {
            KeyIdx::Index(index) => self.storage.get(index.borrow()).map(|(_, value)| value),
            KeyIdx::Key(key) => self
                .indices
                .get(key)
                .map(|index| self.storage.get(index).map(|(_, value)| value))
                .unwrap_or(None),
            KeyIdx::Both { index, .. } => self.storage.get(index.borrow()).map(|(_, value)| value),
        }
    }

    pub fn get_mut<Q, I>(&mut self, ki: &KeyIdx<Q, I>) -> Option<&mut T>
    where
        K: Borrow<Q>,
        I: Borrow<S::Index>,
        Q: Hash + Eq,
    {
        match ki {
            KeyIdx::Index(index) => self.storage.get_mut(index.borrow()).map(|(_, value)| value),
            KeyIdx::Key(key) => match self.indices.get(key) {
                Some(index) => self.storage.get_mut(index).map(|(_, value)| value),
                None => None,
            },
            KeyIdx::Both { index, .. } => {
                self.storage.get_mut(index.borrow()).map(|(_, value)| value)
            }
        }
    }

    pub fn get_by_index<I: Borrow<S::Index>>(&self, index: &I) -> Option<&T> {
        self.storage.get(index.borrow()).map(|(_, value)| value)
    }

    pub fn get_by_index_mut<I: Borrow<S::Index>>(&mut self, index: &I) -> Option<&mut T> {
        self.storage.get_mut(index.borrow()).map(|(_, value)| value)
    }

    pub fn get_by_key<Q>(&self, key: &Q) -> Option<&T>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        match self.indices.get(key) {
            Some(index) => self.storage.get(index).map(|(_, value)| value),
            None => None,
        }
    }

    pub fn get_by_key_mut<Q>(&mut self, key: &Q) -> Option<&mut T>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        match self.indices.get(key) {
            Some(index) => self.storage.get_mut(index).map(|(_, value)| value),
            None => None,
        }
    }

    pub fn get_index<Q>(&self, key: &Q) -> Option<&S::Index>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.indices.get(key)
    }

    pub fn get_key<I>(&self, index: &I) -> Option<&K>
    where
        I: Borrow<S::Index>,
    {
        self.storage.get(index.borrow()).map(|(key, _)| key)
    }

    pub fn fill_key_idx<Q, I>(&self, key_idx: &mut KeyIdx<Q, I>) -> bool
    where
        K: Borrow<Q> + Into<Q>,
        S::Index: Into<I>,
        Q: Hash + Eq,
        I: Borrow<S::Index>,
    {
        let mut result = true;

        take_mut::take(key_idx, |ki| match ki {
            KeyIdx::Key(key) => match self.get_index(&key) {
                Some(index) => KeyIdx::Both {
                    key,
                    index: index.clone().into(),
                },
                None => {
                    result = false;
                    KeyIdx::Key(key)
                }
            },
            KeyIdx::Index(index) => match self.get_key(&index) {
                Some(key) => KeyIdx::Both {
                    key: key.clone().into(),
                    index,
                },
                None => {
                    result = false;
                    KeyIdx::Index(index)
                }
            },
            KeyIdx::Both { key, index } => KeyIdx::Both { key, index },
        });

        result
    }

    pub fn fill_key_idx_get<Q, I>(&self, key_idx: &mut KeyIdx<Q, I>) -> Option<&T>
    where
        K: Borrow<Q> + Into<Q>,
        S::Index: Borrow<I> + Into<I>,
        Q: Hash + Eq,
        I: Hash + Eq + Borrow<S::Index>,
    {
        if !self.fill_key_idx(key_idx) {
            return None;
        }

        self.get(key_idx)
    }

    pub fn insert(&mut self, key: K, value: T) -> (&S::Index, Option<T>) {
        let index = self.storage.insert((key.clone(), value));

        match self.indices.entry(key) {
            HashEntry::Occupied(mut occupied) => {
                let previous = occupied.insert(index);
                let removed = self.storage.remove(&previous).map(|(_, value)| value);
                (occupied.into_mut(), removed)
            }
            HashEntry::Vacant(vacant) => (vacant.insert(index), None),
        }
    }

    pub fn insert_get(&mut self, key: K, value: T) -> (&S::Index, &mut T, Option<T>) {
        let index = self.storage.insert((key.clone(), value));

        match self.indices.entry(key) {
            HashEntry::Occupied(mut occupied) => {
                let previous = occupied.insert(index);
                let removed = self.storage.remove(&previous).map(|(_, value)| value);
                let value = &mut self.storage.get_mut(occupied.get()).unwrap().1;
                (occupied.into_mut(), value, removed)
            }
            HashEntry::Vacant(vacant) => {
                let index = vacant.insert(index);
                let value = &mut self.storage.get_mut(index).unwrap().1;
                (index, value, None)
            }
        }
    }

    pub fn remove<Q, I>(&mut self, ki: &KeyIdx<Q, I>) -> Option<T>
    where
        K: Borrow<Q>,
        I: Borrow<S::Index>,
        Q: Hash + Eq,
    {
        match ki {
            KeyIdx::Index(index) | KeyIdx::Both { index, .. } => {
                self.storage.remove(index.borrow()).map(|(key, value)| {
                    self.indices.remove(key.borrow());
                    value
                })
            }
            KeyIdx::Key(key) => self
                .indices
                .remove(key)
                .map(|idx| self.storage.remove(&idx).map(|(_, value)| value))
                .unwrap_or(None),
        }
    }

    // Iterates in same order as hash map
    pub fn iter<'a>(&'a self) -> impl Iterator<Item = (&'a K, &'a S::Index, &'a T)> + 'a {
        self.indices.iter().map(move |(key, idx)| {
            let value = self.storage.get(idx).unwrap();
            (key, idx, &value.1)
        })
    }

    pub fn values<'a>(&'a self) -> impl Iterator<Item = &'a T> + 'a {
        self.indices.iter().map(move |(_, idx)| {
            let value = self.storage.get(idx).unwrap();
            &value.1
        })
    }

    pub fn values_mut<'a>(&'a mut self) -> impl Iterator<Item = &'a mut T> + 'a {
        let storage = &mut self.storage;

        self.indices.iter().map(move |(_, idx)| {
            let value = storage.get_mut(idx).unwrap();
            let ptr = &mut value.1;
            let ptr = ptr as *mut T;

            // This is annoying WHY RUST
            unsafe { &mut *ptr }
        })
    }
}
