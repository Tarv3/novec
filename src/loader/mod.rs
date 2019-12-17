#[cfg(feature = "json")]
pub mod json_file;

use crate::KeyIdx;
use std::error::Error;

pub trait MappedContainer<K, T> {
    fn insert(&mut self, key: K, item: T) -> usize;
    fn get<Q>(&self, key_idx: KeyIdx<Q>) -> Option<&T>;
    fn get_mut<Q>(&mut self , key_idx: KeyIdx<Q>) -> Option<&mut T>;
    fn remove<Q>(&mut self, key_idx: KeyIdx<Q>) -> Option<(K, T)>;
    // fn for_each<F: FnMut(&K, usize, &T)>(&self, f: F)>;
    // fn for_each_mut<F: FnMut(&K, usize, &mut T)>(&mut self, f: F)>;
    // fn for_each_value<F: FnMut(&T)>(&self, f: F)>;
    // fn for_each_value_mut<F: FnMut(&T)>(&mut self, f: F)>;
}

pub trait Loader<K, T> {
    fn request_load(&mut self, key: K) -> bool;
    fn load_now(&mut self, key: K) -> Result<T, Box<dyn Error>>;
    fn drain_loaded<F: FnMut(T)>(&mut self, f: F);
}