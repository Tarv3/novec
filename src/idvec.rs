use crate::UnorderedStorage;
use derive_deref::{Deref, DerefMut};

#[derive(Copy, Clone, Deref, DerefMut, Debug)]
pub struct IdVecIndex(pub usize);

impl From<usize> for IdVecIndex {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

#[derive(Clone, Debug)]
pub struct IdVec<T> {
    container: Vec<Option<T>>,
}

impl<T> IdVec<T> {
    pub fn new() -> Self {
        Self { container: vec![] }
    }

    pub fn clear(&mut self) {
        self.container.clear();
    }

    pub fn with_capacity(cap: usize) -> Self {
        Self {
            container: Vec::with_capacity(cap),
        }
    }

    pub fn fill_to(&mut self, size: usize) {
        for _ in self.container.len()..size {
            self.container.push(None)
        }
    }

    pub fn insert(&mut self, index: impl Into<IdVecIndex>, value: T) -> Option<T> {
        let index = *index.into();

        if index < self.container.len() {
            return std::mem::replace(&mut self.container[index], Some(value));
        }

        self.fill_to(index + 1);

        std::mem::replace(&mut self.container[index], Some(value))
    }

    pub fn remove(&mut self, index: impl Into<IdVecIndex>) -> Option<T> {
        let index = *index.into();

        if index >= self.container.len() {
            return None;
        }

        std::mem::replace(&mut self.container[index], None)
    }

    pub fn get(&self, index: impl Into<IdVecIndex>) -> Option<&T> {
        let index = *index.into();

        if index >= self.container.len() {
            return None;
        }

        self.container[index].as_ref()
    }

    pub fn get_mut(&mut self, index: impl Into<IdVecIndex>) -> Option<&mut T> {
        let index = *index.into();

        if index >= self.container.len() {
            return None;
        }

        self.container[index].as_mut()
    }

    pub fn iter(&'_ self) -> impl Iterator<Item = (usize, &'_ T)> + '_ {
        self.container
            .iter()
            .enumerate()
            .filter(|(_, value)| value.is_some())
            .map(|(idx, value)| (idx, value.as_ref().unwrap()))
    }

    pub fn iter_mut(&'_ mut self) -> impl Iterator<Item = (usize, &'_ mut T)> + '_ {
        self.container
            .iter_mut()
            .enumerate()
            .filter(|(_, value)| value.is_some())
            .map(|(idx, value)| (idx, value.as_mut().unwrap()))
    }
}

impl<T> UnorderedStorage for IdVec<T> {
    type Index = IdVecIndex;
    type Item = T;

    fn insert(&mut self, index: Self::Index, value: Self::Item) -> Option<Self::Item> {
        IdVec::insert(self, *index, value)
    }

    fn remove(&mut self, index: &Self::Index) -> Option<Self::Item> {
        IdVec::remove(self, **index)
    }

    fn get(&self, index: &Self::Index) -> Option<&Self::Item> {
        IdVec::get(self, **index)
    }

    fn get_mut(&mut self, index: &Self::Index) -> Option<&mut Self::Item> {
        IdVec::get_mut(self, **index)
    }
}

impl<T> Default for IdVec<T> {
    fn default() -> Self {
        IdVec::new()
    }
}
