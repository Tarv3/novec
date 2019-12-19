#[cfg(feature = "json")]
pub mod json_file;

use crate::KeyIdx;
use std::error::Error;

pub trait Loader<K, T> {
    fn request_load(&mut self, key: K) -> bool;
    fn load_now(&mut self, key: K) -> Result<T, Box<dyn Error>>;
    fn drain_loaded<F: FnMut(T)>(&mut self, f: F);
}