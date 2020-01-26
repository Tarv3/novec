use crate::{generation::GenerationStorage, idvec::IdVec, novec::NoVec, *};
use std::{
    borrow::Borrow,
    collections::hash_map::{Entry as HashEntry, HashMap},
    hash::Hash,
};

pub type MappedGeneration<K, T> = MappedStorage<IdVec<K>, GenerationStorage<T>>;
pub type MappedNoVec<K, T> = MappedStorage<IdVec<K>, NoVec<T>>;

pub struct Occupied<'a, K: 'a, T: 'a, I: 'a> {
    key: &'a K,
    index: &'a I,
    value: &'a mut T,
}

impl<'a, K, T, I> Occupied<'a, K, T, I> {
    pub fn get(&self) -> &T {
        self.value
    }

    pub fn get_mut(&mut self) -> &mut T {
        self.value
    }
}

pub struct VacantEntry<'a, K: 'a, S: 'a>
where
    S: ExpandableStorage,
    K: UnorderedStorage,
    K::Item: Hash + Eq,
{
    key: K::Item,
    storage: &'a mut MappedStorage<K, S>,
}

pub enum Entry<'a, K: 'a, S: 'a>
where
    S: ExpandableStorage,
    K: UnorderedStorage,
    K::Item: Hash + Eq,
{
    Occupied(Occupied<'a, K::Item, S::Item, S::Index>),
    Vacant(VacantEntry<'a, K, S>),
}

impl<'a, K: 'a, S: 'a> Entry<'a, K, S>
where
    S: ExpandableStorage,
    K: UnorderedStorage,
    K::Item: Hash + Eq + Clone,
    S::Index: Into<K::Index> + Copy,
    K::Index: Copy,
{
    pub fn key(&self) -> &K::Item {
        match self {
            Entry::Occupied(occupied) => occupied.key,
            Entry::Vacant(vacant) => &vacant.key,
        }
    }

    pub fn or_insert(self, default: S::Item) -> (&'a S::Index, &'a mut S::Item) {
        match self {
            Self::Occupied(occupied) => (occupied.index, occupied.value),
            Self::Vacant(VacantEntry { key, storage }) => {
                let (index, value, _) = storage.insert_get(key, default);

                (index, value)
            }
        }
    }

    pub fn or_insert_with<F: FnOnce() -> S::Item>(
        self,
        default: F,
    ) -> (&'a S::Index, &'a mut S::Item) {
        self.or_insert(default())
    }

    pub fn and_modify<F: FnOnce(&mut S::Item)>(mut self, f: F) -> Self {
        match &mut self {
            Entry::Occupied(Occupied { value, .. }) => f(value),
            _ => {}
        }

        self
    }

    pub fn or_default(self) -> (&'a S::Index, &'a mut S::Item)
    where
        S::Item: Default,
    {
        self.or_insert(Default::default())
    }
}

#[derive(Clone, Debug)]
pub struct MappedStorage<K, S>
where
    S: ExpandableStorage,
    K: UnorderedStorage,
    K::Item: Hash + Eq,
{
    indices: HashMap<K::Item, S::Index>,
    keys: K,
    storage: S,
}

impl<K, S> MappedStorage<K, S>
where
    S: ExpandableStorage + Default,
    K: UnorderedStorage + Default,
    K::Item: Hash + Eq,
{
    pub fn new() -> Self {
        MappedStorage {
            indices: HashMap::new(),
            keys: K::default(),
            storage: S::default(),
        }
    }
}

impl<K, S> MappedStorage<K, S>
where
    S: ExpandableStorage,
    K: UnorderedStorage,
    K::Item: Hash + Eq,
    S::Index: Into<K::Index> + Copy,
    K::Index: Copy,
{
    pub fn entry<Q, I>(&mut self, ki: KeyIdx<Q, I>) -> Option<Entry<K, S>>
    where
        K::Item: Borrow<Q>,
        I: Borrow<S::Index>,
        Q: Hash + Eq + Into<K::Item>,
    {
        match ki {
            KeyIdx::Index(index) | KeyIdx::Both { index, .. } => {
                let key_idx: K::Index = index.borrow().clone().into();
                let value = self.storage.get_mut(index.borrow())?;
                let key = self.keys.get(&key_idx)?;
                let index = self.indices.get((&*key).borrow()).unwrap();

                Some(Entry::Occupied(Occupied { key, index, value }))
            }
            KeyIdx::Key(key) => {
                let self_ptr = self as *mut Self;

                if let Some(index) = self.indices.get(&key) {
                    let value = self.storage.get_mut(index)?;
                    let key = self.keys.get(&index.clone().into())?;
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

    pub fn contains<Q, I>(&self, ki: &KeyIdx<Q, I>) -> bool
    where
        K::Item: Borrow<Q>,
        I: Borrow<S::Index>,
        Q: Hash + Eq,
    {
        self.get(ki).is_some()
    }

    pub fn get<Q, I>(&self, ki: &KeyIdx<Q, I>) -> Option<&S::Item>
    where
        K::Item: Borrow<Q>,
        I: Borrow<S::Index>,
        Q: Hash + Eq,
    {
        match ki {
            KeyIdx::Index(index) | KeyIdx::Both { index, .. } => self.storage.get(index.borrow()),
            KeyIdx::Key(key) => self
                .indices
                .get(key)
                .map(|index| self.storage.get(index))
                .unwrap_or(None),
        }
    }

    pub fn get_mut<Q, I>(&mut self, ki: &KeyIdx<Q, I>) -> Option<&mut S::Item>
    where
        K::Item: Borrow<Q>,
        I: Borrow<S::Index>,
        Q: Hash + Eq,
    {
        match ki {
            KeyIdx::Index(index) => self.storage.get_mut(index.borrow()),
            KeyIdx::Key(key) => match self.indices.get(key) {
                Some(index) => self.storage.get_mut(index),
                None => None,
            },
            KeyIdx::Both { index, .. } => self.storage.get_mut(index.borrow()),
        }
    }

    pub fn get_by_index<I: Borrow<S::Index>>(&self, index: &I) -> Option<&S::Item> {
        self.storage.get(index.borrow())
    }

    pub fn get_by_index_mut<I: Borrow<S::Index>>(&mut self, index: &I) -> Option<&mut S::Item> {
        self.storage.get_mut(index.borrow())
    }

    pub fn get_by_key<Q>(&self, key: &Q) -> Option<&S::Item>
    where
        K::Item: Borrow<Q>,
        Q: Hash + Eq,
    {
        match self.indices.get(key) {
            Some(index) => self.storage.get(index),
            None => None,
        }
    }

    pub fn get_by_key_mut<Q>(&mut self, key: &Q) -> Option<&mut S::Item>
    where
        K::Item: Borrow<Q>,
        Q: Hash + Eq,
    {
        match self.indices.get(key) {
            Some(index) => self.storage.get_mut(index),
            None => None,
        }
    }

    pub fn get_index<Q>(&self, key: &Q) -> Option<&S::Index>
    where
        K::Item: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.indices.get(key)
    }

    pub fn get_key<I>(&self, index: &I) -> Option<&K::Item>
    where
        I: Borrow<S::Index>,
    {
        self.keys.get(&index.borrow().clone().into())
    }

    pub fn fill_key_idx<Q, I>(&self, key_idx: &mut KeyIdx<Q, I>) -> bool
    where
        K::Item: Borrow<Q> + Into<Q> + Clone,
        S::Index: Into<I> + Clone,
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

    pub fn fill_key_idx_get<Q, I>(&self, key_idx: &mut KeyIdx<Q, I>) -> Option<&S::Item>
    where
        K::Item: Borrow<Q> + Into<Q> + Clone,
        S::Index: Borrow<I> + Into<I> + Clone,
        Q: Hash + Eq,
        I: Borrow<S::Index>,
    {
        if !self.fill_key_idx(key_idx) {
            return None;
        }

        self.get(key_idx)
    }

    pub fn fill_key_idx_get_mut<Q, I>(&mut self, key_idx: &mut KeyIdx<Q, I>) -> Option<&mut S::Item>
    where
        K::Item: Borrow<Q> + Into<Q> + Clone,
        S::Index: Borrow<I> + Into<I> + Clone,
        Q: Hash + Eq,
        I: Borrow<S::Index>,
    {
        if !self.fill_key_idx(key_idx) {
            return None;
        }

        self.get_mut(key_idx)
    }

    pub fn insert_replace_idx<Q, I>(
        &mut self,
        key_idx: &mut KeyIdx<Q, I>,
        value: S::Item,
    ) -> Option<S::Item>
    where
        K::Item: Clone,
        S::Index: Borrow<I> + Into<I> + Clone,
        Q: Hash + Eq + Into<K::Item> + Clone,
        I: Borrow<S::Index>,
    {
        if key_idx.is_only_index() {
            return None;
        }

        let mut swapped = None;

        take_mut::take(key_idx, |ki| {
            let key = ki.into_key().unwrap();

            let (index, removed) = self.insert(key.clone().into(), value);
            swapped = removed;

            KeyIdx::Both { key, index: index.clone().into() }
        });
        
        swapped
    }

    pub fn insert(&mut self, key: K::Item, value: S::Item) -> (&S::Index, Option<S::Item>)
    where
        K::Item: Clone,
    {
        let index = self.storage.push(value);
        self.keys.insert(&index.into(), key.clone());

        match self.indices.entry(key) {
            HashEntry::Occupied(mut occupied) => {
                let previous = occupied.insert(index);
                let removed = self.storage.remove(&previous);
                (occupied.into_mut(), removed)
            }
            HashEntry::Vacant(vacant) => (vacant.insert(index), None),
        }
    }

    pub fn insert_get(
        &mut self,
        key: K::Item,
        value: S::Item,
    ) -> (&S::Index, &mut S::Item, Option<S::Item>)
    where
        K::Item: Clone,
    {
        let index = self.storage.push(value);
        self.keys.insert(&index.into(), key.clone());

        match self.indices.entry(key) {
            HashEntry::Occupied(mut occupied) => {
                let previous = occupied.insert(index);
                let removed = self.storage.remove(&previous);
                let value = self.storage.get_mut(occupied.get()).unwrap();
                (occupied.into_mut(), value, removed)
            }
            HashEntry::Vacant(vacant) => {
                let index = vacant.insert(index);
                let value = self.storage.get_mut(index).unwrap();
                (index, value, None)
            }
        }
    }

    pub fn remove<Q, I>(&mut self, ki: &KeyIdx<Q, I>) -> Option<S::Item>
    where
        K::Item: Borrow<Q>,
        I: Borrow<S::Index>,
        Q: Hash + Eq,
    {
        match ki {
            KeyIdx::Index(index) | KeyIdx::Both { index, .. } => {
                self.keys
                    .remove(&index.borrow().clone().into())
                    .map(|key| self.indices.remove(key.borrow()));
                self.storage.remove(index.borrow())
            }
            KeyIdx::Key(key) => self
                .indices
                .remove(key)
                .map(|idx| self.storage.remove(&idx))
                .unwrap_or(None),
        }
    }

    // Iterates in same order as hash map
    pub fn iter<'a>(
        &'a self,
    ) -> impl Iterator<Item = (&'a K::Item, &'a S::Index, &'a S::Item)> + 'a {
        self.indices.iter().map(move |(key, idx)| {
            let value = self.storage.get(idx).unwrap();
            (key, idx, value)
        })
    }

    pub fn values<'a>(&'a self) -> impl Iterator<Item = &'a S::Item> + 'a {
        self.indices.iter().map(move |(_, idx)| {
            let value = self.storage.get(idx).unwrap();
            value
        })
    }

    pub fn values_mut<'a>(&'a mut self) -> impl Iterator<Item = &'a mut S::Item> + 'a {
        let storage = &mut self.storage;

        self.indices.iter().map(move |(_, idx)| {
            let value = storage.get_mut(idx).unwrap();
            let ptr = value as *mut S::Item;

            // why rust
            unsafe { &mut *ptr }
        })
    }
}
