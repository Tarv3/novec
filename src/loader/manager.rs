use super::*;
use std::{collections::HashMap, ops::AddAssign};

pub type ManangedGenSystem<K, L, T, C> =
    ManagedStorage<IdVec<K>, GenerationStorage<GenericPromise<T>>, L, T, C>;
pub type ManagedGen<K, T, C> = ManangedGenSystem<K, GenericSender<K>, T, C>;

pub trait Counter {
    fn zero() -> Self;
    fn is_valid(&self, other: &Self) -> bool;
    fn increment(&mut self, value: &Self);
}

impl Counter for f32 {
    fn zero() -> Self {
        0.0
    }

    fn is_valid(&self, other: &f32) -> bool {
        self < other
    }

    fn increment(&mut self, value: &f32) {
        *self += *value;
    }
}

impl Counter for u32 {
    fn zero() -> Self {
        0
    }

    fn is_valid(&self, other: &u32) -> bool {
        self < other
    }

    fn increment(&mut self, value: &u32) {
        *self += *value;
    }
}

pub struct ManagedStorage<K, S, L, T, C>
where
    S: ExpandableStorage<Item = Promise<T, L::Item>>,
    K: UnorderedStorage,
    K::Item: Hash + Eq,
    S::Index: Into<K::Index> + Copy + Hash + Eq,
    K::Index: Copy,
    L: Loader<Key = K::Item>,
    C: Counter,
{
    storage: StorageSystem<K, S, L, T>,
    pub counters: HashMap<S::Index, C>,
    threshold: C,
}

impl<K, S, L, T, C> ManagedStorage<K, S, L, T, C>
where
    T: 'static,
    S: ExpandableStorage<Item = Promise<T, L::Item>>,
    K: UnorderedStorage,
    K::Item: Hash + Eq,
    S::Index: Into<K::Index> + Copy + Hash + Eq,
    K::Index: Copy,
    L: Loader<Key = K::Item, Meta = TypeId>,
    C: Counter,
{
    pub fn new(storage: StorageSystem<K, S, L, T>, threshold: C) -> Self {
        Self {
            storage,
            counters: HashMap::default(),
            threshold,
        }
    }

    pub fn get(&self, ki: &KeyIdx<K::Item, S::Index>) -> Option<&T> {
        self.storage.get(ki)
    }

    pub fn load(&mut self, ki: &mut KeyIdx<K::Item, S::Index>) -> LoadStatus
    where
        K::Item: Clone,
    {
        self.storage.load(ki)
    }

    pub fn update_loaded(&mut self) -> Result<(), PromiseError>
    where
        L::Item: Convert<T>,
    {
        for (_, idx, value) in self.storage.storage.iter_mut() {
            if let UpdateStatus::Updated = value.update()? {
                self.counters.insert(*idx, C::zero());
            }
        }

        Ok(())
    }

    pub fn update_loaded_blocking(&mut self) -> Result<(), PromiseError>
    where
        L::Item: Convert<T>,
    {
        for (_, idx, value) in self.storage.storage.iter_mut() {
            if let UpdateStatus::Updated = value.update_blocking()? {
                self.counters.insert(*idx, C::zero());
            }
        }

        Ok(())
    }

    pub fn increment(&mut self, inc: &C) {
        let storage = &mut self.storage.storage;
        let counters = &mut self.counters;
        let threshold = &self.threshold;

        storage.retain(|idx, _| {
            let mut result = true;
            if let Some(value) = counters.get_mut(idx) {
                value.increment(inc);
                result = value.is_valid(threshold);
            }

            if !result {
                counters.remove(idx);
            }

            result
        });
    }

    pub fn remove_out_of_date(&mut self) {
        let storage = &mut self.storage.storage;
        let counters = &mut self.counters;
        let threshold = &self.threshold;

        storage.retain(|idx, _| {
            let mut result = true;
            if let Some(value) = counters.get(idx) {
                result = value.is_valid(threshold);
            }

            if !result {
                counters.remove(idx);
            }

            result
        });
    }
}
