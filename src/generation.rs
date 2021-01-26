use crate::{idvec::IdVecIndex, *};

#[derive(Copy, Clone, Debug, PartialEq, Hash, Eq)]
pub struct StorageId {
    pub index: usize,
    pub generation: u64,
}

impl Into<IdVecIndex> for StorageId {
    fn into(self) -> IdVecIndex {
        IdVecIndex(self.index)
    }
}

#[derive(Copy, Clone, Debug)]
pub struct StorageObject<T> {
    generation: u64,
    item: Option<T>,
}

impl<T> StorageObject<T> {
    pub fn new(item: T) -> StorageObject<T> {
        StorageObject { item: Some(item), generation: 0 }
    }

    pub fn empty(generation: u64) -> StorageObject<T> {
        StorageObject { generation, item: None }
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn increase_generation(&mut self) {
        self.generation = self.generation.wrapping_add(1);
    }

    pub fn is_some(&self) -> bool {
        self.item.is_some()
    }

    pub fn is_none(&self) -> bool {
        self.item.is_none()
    }

    pub fn item(&self) -> Option<&T> {
        self.item.as_ref()
    }

    pub fn item_mut(&mut self) -> Option<&mut T> {
        self.item.as_mut()
    }

    pub fn unwrap(self) -> T {
        self.item.unwrap()
    }

    pub fn unwrap_ref(&self) -> &T {
        match &self.item {
            Some(item) => item,
            _ => panic!("Tried to unwrap None storage object"),
        }
    }

    pub fn unwrap_mut(&mut self) -> &mut T {
        match &mut self.item {
            Some(item) => item,
            _ => panic!("Tried to unwrap None storage object"),
        }
    }

    pub fn remove(&mut self) -> Option<T> {
        self.item.take()
    }

    // Returns the contained value if there was any
    pub fn insert(&mut self, item: T) -> Option<T> {
        let to_return = self.item.take();
        self.item = Some(item);

        to_return
    }
}

#[derive(Clone, Debug)]
pub struct GenerationStorage<T> {
    objects: Vec<StorageObject<T>>,
    available: Vec<usize>,
}

impl<T> Default for GenerationStorage<T> {
    fn default() -> Self {
        GenerationStorage::new()
    }
}

impl<T> GenerationStorage<T> {
    pub fn new() -> GenerationStorage<T> {
        GenerationStorage { objects: vec![], available: vec![] }
    }

    // Returns what index would be given to an object after n insertions if no deletion occur
    pub fn nth_available(&self, n: usize) -> StorageId {
        if n < self.available.len() {
            let index = self.available[self.available.len() - 1 - n];
            let generation = self.objects[index].generation + 1;

            return StorageId { index, generation };
        }

        let overflow = n - self.available.len();
        let index = self.objects.len() + overflow;

        StorageId { index, generation: 0 }
    }

    pub fn clear(&mut self) {
        for (i, item) in self.objects.iter_mut().filter(|item| item.is_some()).enumerate() {
            item.remove();
            self.available.push(i);
        }
    }

    pub fn insert(&mut self, id: StorageId, item: T) -> Option<T> {
        if id.index >= self.objects.len() {
            self.fill_to(id.index + 1);
            self.available.pop();
            let object = &mut self.objects[id.index];
            object.item = Some(item);
            object.generation = id.generation;

            return None;
        }

        let object = &mut self.objects[id.index];

        if object.is_none() {
            if let Some(position) = self.available.iter().position(|a| *a == id.index) {
                self.available.swap_remove(position);
            }
        }

        object.item = Some(item);
        object.generation = id.generation;

        None
    }

    pub fn push(&mut self, item: T) -> StorageId {
        match self.available.pop() {
            Some(id) => {
                self.objects[id].increase_generation();
                self.objects[id].insert(item);

                StorageId { index: id, generation: self.objects[id].generation() }
            }
            None => {
                let id = self.objects.len();
                let object = StorageObject::new(item);
                self.objects.push(object);

                StorageId { index: id, generation: 0 }
            }
        }
    }

    pub fn remove(&mut self, id: usize) -> Option<T> {
        if id < self.objects.len() {
            if self.objects[id].is_some() {
                self.available.push(id);
            }

            return self.objects[id].remove();
        }

        None
    }

    pub fn retain<F: FnMut(&T) -> bool>(&mut self, mut f: F) {
        for (id, object) in self.objects.iter_mut().enumerate() {
            match &object.item {
                Some(item) => {
                    if !f(item) {
                        object.remove();
                        self.available.push(id);
                    }
                }
                None => {}
            }
        }
    }

    pub fn remove_id(&mut self, id: StorageId) -> Option<T> {
        if self.contains(id) {
            return self.remove(id.index);
        }

        None
    }

    pub fn contains(&self, id: StorageId) -> bool {
        self.get(id).is_some()
    }

    pub fn get(&self, id: StorageId) -> Option<&T> {
        if id.index >= self.objects.len() {
            return None;
        }

        let object = &self.objects[id.index];

        if object.is_some() && object.generation == id.generation {
            return object.item.as_ref();
        }

        None
    }

    pub fn get_mut(&mut self, id: StorageId) -> Option<&mut T> {
        if id.index >= self.objects.len() {
            return None;
        }

        let object = &mut self.objects[id.index];

        if object.is_some() && object.generation == id.generation {
            return object.item.as_mut();
        }

        None
    }

    pub fn get_unchecked(&self, idx: usize) -> Option<&T> {
        self.objects.get(idx).map(|value| value.item.as_ref()).flatten()
    }

    pub fn get_mut_unchecked(&mut self, idx: usize) -> Option<&mut T> {
        self.objects.get_mut(idx).map(|value| value.item.as_mut()).flatten()
    }

    pub fn fill_to(&mut self, size: usize) {
        for i in self.objects.len()..size {
            self.objects.push(StorageObject::empty(0));
            self.available.push(i);
        }
    }

    pub fn values<'a>(&'a self) -> impl Iterator<Item = &'a T> + 'a {
        self.objects.iter().filter(|x| x.is_some()).map(|x| x.unwrap_ref())
    }

    pub fn values_mut<'a>(&'a mut self) -> impl Iterator<Item = &'a mut T> + 'a {
        self.objects.iter_mut().filter(|x| x.is_some()).map(|x| x.unwrap_mut())
    }

    pub fn iter<'a>(&'a self) -> impl Iterator<Item = &StorageObject<T>> + 'a {
        self.objects.iter().filter(|x| x.is_some())
    }

    pub fn iter_mut<'a>(&'a mut self) -> impl Iterator<Item = &mut StorageObject<T>> + 'a {
        self.objects.iter_mut().filter(|x| x.is_some())
    }

    pub fn iter_with_ids<'a>(&'a self) -> impl Iterator<Item = (StorageId, &'a T)> + 'a {
        self.objects.iter().enumerate().filter(|(_, x)| x.is_some()).map(|(i, x)| {
            let generation = x.generation();
            let id = StorageId { index: i, generation };

            (id, x.unwrap_ref())
        })
    }

    pub fn iter_with_ids_mut<'a>(
        &'a mut self,
    ) -> impl Iterator<Item = (StorageId, &'a mut T)> + 'a {
        self.objects.iter_mut().enumerate().filter(|(_, x)| x.is_some()).map(|(i, x)| {
            let generation = x.generation();
            let id = StorageId { index: i, generation };

            (id, x.unwrap_mut())
        })
    }
}

impl<T> UnorderedStorage for GenerationStorage<T> {
    type Index = StorageId;
    type Item = T;

    fn insert(&mut self, index: StorageId, value: T) -> Option<T> {
        <GenerationStorage<T>>::insert(self, index, value)
    }

    fn remove(&mut self, index: &StorageId) -> Option<T> {
        self.remove_id(*index)
    }

    fn get(&self, index: &StorageId) -> Option<&T> {
        <GenerationStorage<T>>::get(self, *index)
    }

    fn get_mut(&mut self, index: &StorageId) -> Option<&mut T> {
        <GenerationStorage<T>>::get_mut(self, *index)
    }
}

impl<T> ExpandableStorage for GenerationStorage<T> {
    fn push(&mut self, value: T) -> StorageId {
        self.push(value)
    }
}