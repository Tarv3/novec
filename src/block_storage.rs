use std::{
    cell::UnsafeCell,
    collections::HashSet,
    hash::Hash,
    mem::MaybeUninit,
    ops::{Index, IndexMut},
};

/// This is designed to act as a unique key in to the block storage that can only be created by
/// the blockstorage.
/// NOTE: It may not be unique if multiple 'BlockStorage' objects exists
#[derive(Debug)]
pub struct BlockKey {
    idx: usize,
    blocks: usize,
    generation: usize,
}

#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
struct InternalBlockKey {
    idx: usize,
    blocks: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockIdx {
    /// Stores the number of elements that are initialized from the start of this block
    OwnedStart(usize),
    Owned,
    /// Stores the number of blocks after this block that are also empty
    EmptyStart(usize),
    Emtpy,
}

impl BlockIdx {
    pub fn is_empty_start(&self) -> bool {
        match self {
            BlockIdx::EmptyStart(_) => true,
            _ => false,
        }
    }

    pub fn get_empty_count(&self) -> usize {
        match self {
            BlockIdx::EmptyStart(size) => *size,
            _ => panic!("Tried to get size of non start block"),
        }
    }

    pub fn get_allocated_count(&self) -> usize {
        match self {
            BlockIdx::OwnedStart(size) => *size,
            _ => panic!("Tried to get size of non start block"),
        }
    }

    pub fn is_owned_start(&self) -> bool {
        match self {
            BlockIdx::OwnedStart(_) => true,
            _ => false,
        }
    }

    pub fn get_allocated_count_mut(&mut self) -> &mut usize {
        match self {
            BlockIdx::OwnedStart(size) => size,
            _ => panic!("Tried to get size of non start block"),
        }
    }
}

pub struct Block<'a, T> {
    key: BlockKey,
    len: &'a mut usize,
    data: &'a mut [MaybeUninit<T>],
}

impl<'a, T> Block<'a, T> {
    pub fn return_key(self) -> BlockKey {
        self.key
    }

    pub fn len(&self) -> usize {
        *self.len
    }

    pub fn push(&mut self, item: T) -> Option<T> {
        if *self.len >= self.data.len() {
            return Some(item);
        }

        self.data[*self.len] = MaybeUninit::new(item);
        *self.len += 1;

        None
    }

    pub fn pop(&mut self) -> Option<T> {
        if *self.len == 0 {
            return None;
        }

        let value = std::mem::replace(&mut self.data[*self.len], MaybeUninit::uninit());
        let value = unsafe { value.assume_init() };

        *self.len -= 1;

        Some(value)
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        if index >= *self.len {
            return None;
        }

        let value = unsafe { &*self.data[index].as_ptr() };
        Some(value)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        if index >= *self.len {
            return None;
        }

        let value = unsafe { &mut *self.data[index].as_mut_ptr() };
        Some(value)
    }

    pub fn as_slice(&self) -> &[T] {
        let ptr = self.data[0].as_ptr();

        unsafe { std::slice::from_raw_parts(ptr, *self.len) }
    }

    pub fn as_slice_mut(&mut self) -> &mut [T] {
        let ptr = self.data[0].as_mut_ptr();

        unsafe { std::slice::from_raw_parts_mut(ptr, *self.len) }
    }
}

impl<'a, T> Index<usize> for Block<'a, T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        self.get(index).unwrap()
    }
}

impl<'a, T> IndexMut<usize> for Block<'a, T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.get_mut(index).unwrap()
    }
}

pub struct BlockStorage<T> {
    block_size: usize,

    generation: usize,
    active_keys: HashSet<InternalBlockKey>,
    blocks: UnsafeCell<Vec<BlockIdx>>,
    data: UnsafeCell<Vec<MaybeUninit<T>>>,
}

impl<T> Drop for BlockStorage<T> {
    fn drop(&mut self) {
        self.clear_data();
    }
}

impl<T> BlockStorage<T> {
    pub fn new(block_size: usize) -> Self {
        Self {
            block_size,
            generation: 0,
            active_keys: HashSet::new(),
            blocks: UnsafeCell::new(vec![]),
            data: UnsafeCell::new(vec![]),
        }
    }

    fn clear_data(&mut self) {
        let blocks = unsafe { &mut *self.blocks.get() };
        let data = unsafe { &mut *self.data.get() };

        for (i, block) in blocks.iter().enumerate().filter(|(_, block)| block.is_owned_start()) {
            let idx = i * self.block_size;
            let allocated = block.get_allocated_count();

            for value in data[idx..idx + allocated].iter_mut() {
                let value = std::mem::replace(value, MaybeUninit::uninit());
                unsafe { value.assume_init() };
            }
        }

        blocks.clear();
        data.clear();
    }

    pub fn clear(&mut self) {
        self.generation += 1;
        self.clear_data();
        self.active_keys.clear();
    }

    fn push_empty_blocks(&mut self, to_insert: usize) {
        unsafe {
            // We have a mutable reference to self so this is allowed
            let blocks = &mut *self.blocks.get();
            let data = &mut *self.data.get();

            for _ in 0..to_insert {
                blocks.push(BlockIdx::Emtpy);

                for _ in 0..self.block_size {
                    data.push(MaybeUninit::uninit());
                }
            }
        }
    }

    pub fn get(&self, key: BlockKey) -> Option<Block<T>> {
        if key.generation != self.generation {
            return None;
        }

        // If no two keys can point to the same blocks then this is safe
        unsafe {
            let blocks = &mut *self.blocks.get();
            let data = &mut *self.data.get();

            // This is a unique reference if 'key.idx' is unique
            let len = blocks[key.idx].get_allocated_count_mut();
            let start = key.idx * self.block_size;
            let size = key.blocks * self.block_size;

            // This is a unique reference as 'start' is unqiue when 'key.idx' is unique and 'size'
            // should have been determined during the creation of this key
            let slice = &mut data[start..start + size];

            Some(Block { key, len, data: slice })
        }
    }

    pub fn create(&mut self, size: usize) -> BlockKey {
        if size == 0 {
            panic!("Tried to create empty block");
        }

        let required_blocks = size / self.block_size + (size % self.block_size > 0) as usize;
        let blocks = unsafe { &mut *self.blocks.get() };

        let mut block_id = None;
        let mut min_diff = None;

        // Search for the smallest block that can fit the required size
        for (i, block) in blocks.iter().enumerate().filter(|(_, block)| block.is_empty_start()) {
            let size = block.get_empty_count() + 1;

            if size < required_blocks {
                continue;
            }

            let diff = size - required_blocks;

            if diff == 0 {
                block_id = Some(i);
                break;
            }

            if Some(diff) < min_diff {
                min_diff = Some(diff);
                block_id = Some(i);
            }
        }

        let block_id = match block_id {
            Some(id) => id,
            // There was not a large enough block so we create a new one
            None => {
                let id = blocks.len();
                self.push_empty_blocks(required_blocks);
                blocks[id] = BlockIdx::EmptyStart(required_blocks - 1);

                id
            }
        };

        let start = blocks[block_id];
        let empty_count = start.get_empty_count();
        let next_start = block_id + required_blocks;

        if let Some(BlockIdx::Emtpy) = blocks.get(next_start) {
            blocks[next_start] = BlockIdx::EmptyStart(empty_count - required_blocks);
        }

        blocks[block_id] = BlockIdx::OwnedStart(0);

        for i in 1..required_blocks {
            let owned_id = block_id + i;
            blocks[owned_id] = BlockIdx::Owned;
        }

        let internal = InternalBlockKey { idx: block_id, blocks: required_blocks };
        self.active_keys.insert(internal);

        BlockKey { idx: block_id, blocks: required_blocks, generation: self.generation }
    }
}
