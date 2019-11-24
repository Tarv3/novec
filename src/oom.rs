use std::{slice::IterMut, iter::{once, Once}};


#[derive(Clone, Debug)]
pub enum OneOrMany<T> {
    None,
    One(T),
    Many(Vec<T>),
}

impl<T> OneOrMany<T> {
    pub fn new(mut items: impl Iterator<Item = T>) -> OneOrMany<T> {
        let first = match items.next() {
            Some(item) => item,
            None => return OneOrMany::None,
        };

        let mut vec = match items.next() {
            Some(item) => vec![first, item],
            None => return OneOrMany::One(first)
        };

        for item in items {
            vec.push(item);
        }

        OneOrMany::Many(vec)
    }

    pub fn take_one(self) -> T {
        match self {
            OneOrMany::One(value) => value,
            _ => panic!("Tried to take One value"),
        }
    }

    pub fn push(&mut self, item: T) {
        match self {
            OneOrMany::None => *self = OneOrMany::One(item),
            OneOrMany::One(_) => {
                let temp = std::mem::replace(self, OneOrMany::None);
                *self = OneOrMany::Many(vec![temp.take_one(), item]);
            },
            OneOrMany::Many(vec) => vec.push(item),
        }
    }

    pub fn iter(&self) -> OneOrManyIter<T> {
        OneOrManyIter {
            index: 0,
            values: &self
        }
    }

    pub fn iter_mut(&mut self) -> OneOrManyIterMut<T> {
        match self {
            OneOrMany::None => OneOrManyIterMut::None,
            OneOrMany::One(item) => OneOrManyIterMut::One(once(item)),
            OneOrMany::Many(vec) => OneOrManyIterMut::Many(vec.iter_mut()),
        }
    }
}

pub struct OneOrManyIter<'a, T> {
    index: usize,
    values: &'a OneOrMany<T>
}

impl<'a, T> Iterator for OneOrManyIter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        match self.values {
            OneOrMany::None => None, 
            OneOrMany::One(value) => match self.index {
                0 => {
                    self.index += 1;
                    Some(value)
                },
                _ => None
            }
            OneOrMany::Many(vec) => {
                let to_return = vec.get(self.index);
                self.index += 1;
                to_return
            }
        }
    }
}

pub enum OneOrManyIterMut<'a, T> {
    None,
    One(Once<&'a mut T>), 
    Many(IterMut<'a, T>),
}

impl<'a, T> Iterator for OneOrManyIterMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<&'a mut T> {
        match self {
            OneOrManyIterMut::None => None, 
            OneOrManyIterMut::One(iter) => iter.next(), 
            OneOrManyIterMut::Many(iter) => iter.next(), 
        }
    }
}