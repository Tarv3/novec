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
    pub fn contains(&self, ki: &KeyIdx<K::Item, S::Index>) -> bool {
        if let Some(value) = ki.index_ref() {
            return self.storage.get(value).is_some();
        }

        self.indices.contains_key(&ki.key)
    }

    pub fn get(&self, ki: &KeyIdx<K::Item, S::Index>) -> Option<&S::Item> {
        if let Some(value) = ki.index_ref() {
            return self.storage.get(value);
        }

        self.indices
            .get(&ki.key)
            .map(|index| self.storage.get(index))
            .flatten()
    }

    pub fn get_mut(&mut self, ki: &KeyIdx<K::Item, S::Index>) -> Option<&mut S::Item> {
        if let Some(index) = ki.index_ref() {
            return self.storage.get_mut(index);
        }

        if let Some(index) = self.indices.get(&ki.key) {
            return self.storage.get_mut(index);
        }

        None
    }

    pub fn get_by_index(&self, index: &S::Index) -> Option<&S::Item> {
        self.storage.get(index)
    }

    pub fn get_by_index_mut(&mut self, index: &S::Index) -> Option<&mut S::Item> {
        self.storage.get_mut(index)
    }

    pub fn get_by_key<Q>(&self, key: &Q) -> Option<&S::Item>
    where
        K::Item: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        match self.indices.get(key) {
            Some(index) => self.storage.get(index),
            None => None,
        }
    }

    pub fn get_by_key_mut<Q>(&mut self, key: &Q) -> Option<&mut S::Item>
    where
        K::Item: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        match self.indices.get(key) {
            Some(index) => self.storage.get_mut(index),
            None => None,
        }
    }

    pub fn get_index<Q>(&self, key: &Q) -> Option<&S::Index>
    where
        K::Item: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.indices.get(key)
    }

    pub fn get_key(&self, index: &S::Index) -> Option<&K::Item> {
        self.keys.get(&index.clone().into())
    }

    pub fn fill_key_idx(&self, ki: &mut KeyIdx<K::Item, S::Index>) -> bool {
        match self.get_index(&ki.key) {
            Some(value) => {
                ki.index = Some(*value);
                return true;
            }
            None => return false,
        }
    }

    pub fn fill_key_idx_get(&self, ki: &mut KeyIdx<K::Item, S::Index>) -> Option<&S::Item> {
        if !self.fill_key_idx(ki) {
            return None;
        }

        self.get_by_index(ki.index_ref().unwrap())
    }

    pub fn fill_key_idx_get_mut(
        &mut self,
        ki: &mut KeyIdx<K::Item, S::Index>,
    ) -> Option<&mut S::Item> {
        if !self.fill_key_idx(ki) {
            return None;
        }

        self.get_by_index_mut(ki.index_ref().unwrap())
    }

    pub fn insert_replace_idx(
        &mut self,
        ki: &mut KeyIdx<K::Item, S::Index>,
        value: S::Item,
    ) -> Option<S::Item>
    where
        K::Item: Clone,
    {
        let (index, removed) = self.insert(ki.key.clone(), value);
        ki.index = Some(index);

        removed
    }

    pub fn insert(&mut self, key: K::Item, value: S::Item) -> (S::Index, Option<S::Item>)
    where
        K::Item: Clone,
    {
        let index = self.storage.push(value);
        self.keys.insert(&index.into(), key.clone());

        match self.indices.entry(key) {
            HashEntry::Occupied(mut occupied) => {
                let previous = occupied.insert(index);
                let removed = self.storage.remove(&previous);
                (*occupied.into_mut(), removed)
            }
            HashEntry::Vacant(vacant) => (*vacant.insert(index), None),
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

    pub fn remove_with_index(&mut self, index: &S::Index) -> Option<S::Item> {
        self.keys
            .remove(&(*index).into())
            .map(|key| self.indices.remove(&key));
        return self.storage.remove(index);
    }

    pub fn remove(&mut self, ki: &KeyIdx<K::Item, S::Index>) -> Option<S::Item> {
        if let Some(&index) = ki.index_ref() {
            self.keys
                .remove(&index.into())
                .map(|key| self.indices.remove(key.borrow()));
            return self.storage.remove(&index);
        }

        self.indices
            .remove(&ki.key)
            .map(|idx| self.storage.remove(&idx))
            .flatten()
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

    pub fn iter_mut<'a>(
        &'a mut self,
    ) -> impl Iterator<Item = (&'a K::Item, &'a S::Index, &'a mut S::Item)> + 'a {
        let indices = &self.indices;
        let values = &mut self.storage;

        indices.iter().map(move |(key, idx)| {
            let value = values.get_mut(idx).unwrap();

            // TODO: Remove this unsafe code.
            // Not sure if this is needed or not
            let value = unsafe {
                let ptr = value as *mut S::Item;
                &mut *ptr
            };
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

            // TODO: Remove this unsafe code.
            // why rust
            unsafe { &mut *ptr }
        })
    }

    pub fn indices<'a>(&'a self) -> impl Iterator<Item = (&'a K::Item, &'a S::Index)> + 'a {
        self.indices.iter()
    }

    pub fn retain(&mut self, mut f: impl FnMut(&S::Index, &S::Item) -> bool) {
        let indices = &mut self.indices;
        let keys = &mut self.keys;
        let values = &mut self.storage;

        indices.retain(|_, value| {
            let item = match values.get(value) {
                Some(item) => item,
                None => {
                    keys.remove(&(*value).into());
                    return false;
                }
            };

            if !f(value, item) {
                keys.remove(&(*value).into());
                values.remove(value);
                return false;
            }

            true
        })
    }
}
