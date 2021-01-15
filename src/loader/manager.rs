use super::*;
use std::any::TypeId;

pub type ManangedGenSystem<K, L, T, C> =
    ManagedStorage<IdVec<K>, GenerationStorage<GenericPromise<T>>, L, T, IdVec<C>>;
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
    S::Index: Into<K::Index> + Copy + Hash + Eq,
    K: UnorderedStorage,
    K::Item: Hash + Eq,
    K::Index: Copy,
    C: UnorderedStorage<Index = K::Index>,
    C::Item: Counter,
    L: Loader<Key = K::Item>,
    L::Item: TryInto<T>,
{
    storage: StorageSystem<K, S, L, T>,
    counters: C,
    threshold: C::Item,
}

impl<K, S, L, T, C> ManagedStorage<K, S, L, T, C>
where
    T: 'static,
    S: ExpandableStorage<Item = Promise<T, L::Item>>,
    S::Index: Into<K::Index> + Copy + Hash + Eq,
    K: UnorderedStorage,
    K::Item: Hash + Eq + Clone,
    K::Index: Copy,
    C: UnorderedStorage<Index = K::Index>,
    C::Item: Counter,
    L: Loader<Key = K::Item, Meta = TypeId>,
    L::Item: TryInto<T>,
{
    pub fn new(storage: StorageSystem<K, S, L, T>, threshold: C::Item) -> Self
    where
        C: Default,
    {
        Self { storage, counters: C::default(), threshold }
    }

    pub fn new_with_loader(loader: L, threshold: C::Item) -> Self
    where
        S: Default,
        K: Default,
        C: Default,
    {
        Self { storage: StorageSystem::new_with_loader(loader), counters: C::default(), threshold }
    }

    pub fn get(&self, ki: &KeyIdx<K::Item, S::Index>) -> Option<&T> {
        self.storage.get(ki)
    }

    pub fn get_by_index(&self, idx: &S::Index) -> Option<&T> {
        self.storage.get_by_index(idx)
    }

    pub fn set_idx(&self, ki: &mut KeyIdx<K::Item, S::Index>) -> bool {
        self.storage.set_idx(ki)
    }

    pub fn set_idx_is_loaded(&self, ki: &mut KeyIdx<K::Item, S::Index>) -> bool {
        self.storage.set_idx_is_loaded(ki)
    }

    pub fn set_idx_get_status(&self, ki: &mut KeyIdx<K::Item, S::Index>) -> Option<LoadStatus> {
        self.storage.set_idx_get_status(ki)
    }

    pub fn get_status(&self, ki: &KeyIdx<K::Item, S::Index>) -> Option<LoadStatus> {
        self.storage.get_status(ki)
    }

    pub fn reset_counter(&mut self, idx: S::Index) {
        if let Some(value) = self.counters.get_mut(&idx.into()) {
            *value = C::Item::zero();
        }
    }

    pub fn load(&mut self, ki: &mut KeyIdx<K::Item, S::Index>) -> LoadStatus
    where
        K::Item: Clone,
    {
        self.storage.load(ki)
    }

    pub fn update_loaded(&mut self)
    where
        L::Item: TryInto<T>,
    {
        let storage = &mut self.storage;
        let counters = &mut self.counters;

        storage.on_update_loaded(|_, idx, _| {
            counters.insert((*idx).into(), C::Item::zero());
        });
    }

    pub fn update_loaded_blocking(&mut self)
    where
        L::Item: TryInto<T>,
    {
        let storage = &mut self.storage;
        let counters = &mut self.counters;

        storage.on_update_loaded_blocking(|_, idx, _| {
            counters.insert((*idx).into(), C::Item::zero());
        });
    }

    pub fn on_update_loaded(&mut self, mut f: impl FnMut(&K::Item, &S::Index, &T))
    where
        L::Item: TryInto<T>,
    {
        let storage = &mut self.storage;
        let counters = &mut self.counters;

        storage.on_update_loaded(|key, idx, value| {
            counters.insert((*idx).into(), C::Item::zero());
            f(key, idx, value);
        });
    }

    pub fn on_update_loaded_blocking(&mut self, mut f: impl FnMut(&K::Item, &S::Index, &T))
    where
        L::Item: TryInto<T>,
    {
        let storage = &mut self.storage;
        let counters = &mut self.counters;

        storage.on_update_loaded_blocking(|key, idx, value| {
            counters.insert((*idx).into(), C::Item::zero());
            f(key, idx, value);
        });
    }

    pub fn remove_failed<'a>(
        &'a mut self,
    ) -> impl Iterator<Item = (K::Item, S::Index, PromiseError<<L::Item as TryInto<T>>::Error>)> + 'a
    {
        self.storage.remove_failed()
    }

    pub fn increment(&mut self, inc: &C::Item) {
        let storage = &mut self.storage.storage;
        let counters = &mut self.counters;

        for (_, idx, _) in storage.iter_mut() {
            let counter_idx = (*idx).into();

            if let Some(value) = counters.get_mut(&counter_idx) {
                value.increment(inc);
            }
        }
    }

    pub fn remove_out_of_date(&mut self) {
        let storage = &mut self.storage.storage;
        let counters = &mut self.counters;
        let threshold = &self.threshold;

        storage.retain(|_, idx, _| {
            let c_idx = (*idx).into();
            let mut result = true;
            if let Some(value) = counters.get(&c_idx) {
                result = value.is_valid(threshold);
            }

            if !result {
                counters.remove(&c_idx);
            }

            result
        });
    }

    pub fn on_remove_out_of_date(&mut self, mut f: impl FnMut(&K::Item, &S::Index, &mut S::Item)) {
        let storage = &mut self.storage.storage;
        let counters = &mut self.counters;
        let threshold = &self.threshold;

        storage.retain(|key, idx, item| {
            let c_idx = (*idx).into();
            let mut result = true;
            if let Some(value) = counters.get(&c_idx) {
                result = value.is_valid(threshold);
            }

            if !result {
                counters.remove(&c_idx);

                f(key, idx, item);
            }

            result
        });
    }
}
