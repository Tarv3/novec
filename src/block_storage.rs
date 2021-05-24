use std::{
    cell::UnsafeCell,
    collections::{BTreeSet, HashSet},
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

impl PartialOrd for InternalBlockKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.idx.partial_cmp(&other.idx)
    }
}

impl Ord for InternalBlockKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.idx.cmp(&other.idx)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockIdx {
    /// Stores the number of elements that are initialized from the start of this block
    OwnedStart(usize),
    /// Stores the start of this empty block
    Owned(usize),
    /// Stores the number of blocks after this block that are also empty
    EmptyStart(usize),
    /// Stores the start of this empty block
    Emtpy(usize),
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

    pub fn capacity(&self) -> usize {
        self.data.len()
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
    available_blocks: BTreeSet<usize>,
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
            available_blocks: BTreeSet::new(),
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
        self.available_blocks.clear();
    }

    /// Pushes empty blocks until the last block contains 'size' number of blocks
    fn push_empty_blocks_until(&mut self, size: usize) -> InternalBlockKey {
        let blocks;
        let data;

        // We have a mutable reference to self so this is allowed
        unsafe {
            blocks = &mut *self.blocks.get();
            data = &mut *self.data.get();
        }

        let (parent, empty_size) = match blocks.last() {
            Some(BlockIdx::Emtpy(parent)) => (*parent, blocks[*parent].get_empty_count()),
            Some(BlockIdx::EmptyStart(count)) => (blocks.len() - 1, *count), 
            _ => (blocks.len(), 0),
        };

        for _ in empty_size..size {
            blocks.push(BlockIdx::Emtpy(parent));

            for _ in 0..self.block_size {
                data.push(MaybeUninit::uninit());
            }
        }

        blocks[parent] = BlockIdx::EmptyStart(size);

        InternalBlockKey { idx: parent, blocks: size }
    }

    pub fn get_len(&self, key: &BlockKey) -> Option<usize> {
        if key.generation != self.generation {
            return None;
        }

        // If no two keys can point to the same blocks then this is safe
        unsafe {
            let blocks = &*self.blocks.get();

            // This is a unique reference if 'key.idx' is unique
            let len = blocks[key.idx].get_allocated_count();

            Some(len)
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

    pub fn remove(&mut self, key: BlockKey) {
        if key.generation != self.generation {
            return;
        }

        let blocks;
        let data;

        // If no two keys can point to the same blocks then this is safe
        unsafe {
            blocks = &mut *self.blocks.get();
            data = &mut *self.data.get();
        }

        match blocks[key.idx] {
            BlockIdx::Owned(_) | BlockIdx::EmptyStart(_) | BlockIdx::Emtpy(_) => return,
            BlockIdx::OwnedStart(_) => {}
        }

        let start = key.idx * self.block_size;
        let allocated = blocks[key.idx].get_allocated_count();

        // Deallocate the values
        for value in data[start..start + allocated].iter_mut() {
            let value = std::mem::replace(value, MaybeUninit::uninit());
            unsafe { value.assume_init() };
        }

        let next_block = key.idx + key.blocks;

        let end = match blocks.get(next_block) {
            // If the next block is an empty block then it must be an empty start and we can combine
            // it into this emtpy block
            Some(BlockIdx::EmptyStart(count)) => {
                self.available_blocks.remove(&next_block);
                next_block + count
            }
            _ => next_block
        };

        let start = match key.idx {
            // Check if the previous block is emtpy
            x if x > 0 => match blocks.get(x - 1) {
                // If previous block is empty then the new parent for this block will be that 
                // block's parent
                Some(BlockIdx::Emtpy(parent)) => {
                    self.available_blocks.remove(parent);
                    *parent
                },
                Some(BlockIdx::EmptyStart(_)) => {
                    let parent = x - 1;
                    self.available_blocks.remove(&parent);
                    parent
                }
                _ => x,
            }
            _ => key.idx
        };

        let count = end - start; 
        blocks[start] = BlockIdx::EmptyStart(count);

        for i in 1..count {
            blocks[start + i] = BlockIdx::Emtpy(start);
        }

        self.available_blocks.insert(start);
    }

    pub fn create(&mut self, size: usize) -> BlockKey {
        if size == 0 {
            panic!("Tried to create empty block");
        }

        let required_blocks = size / self.block_size + (size % self.block_size > 0) as usize;
        let blocks = unsafe { &mut *self.blocks.get() };

        let mut block_id = None;
        let mut min_diff = None;

        for block_idx in self.available_blocks.iter() {
            let block = blocks[*block_idx];
            let size = block.get_empty_count();

            if size < required_blocks {
                continue;
            }

            let diff = size - required_blocks;

            if diff == 0 {
                block_id = Some(*block_idx);
                break;
            }

            if Some(diff) < min_diff {
                min_diff = Some(diff);
                block_id = Some(*block_idx);
            }
        }

        let block_id = match block_id {
            Some(id) => id,
            // There was not a large enough block so we create a new one
            None => {
                let id = self.push_empty_blocks_until(required_blocks);
                id.idx
            }
        };

        self.available_blocks.remove(&block_id);

        let start = blocks[block_id];
        let empty_count = start.get_empty_count();

        if empty_count > required_blocks {
            let idx = block_id + required_blocks;
            let block_count = empty_count - required_blocks;

            blocks[idx] = BlockIdx::EmptyStart(block_count);

            for i in 1..block_count {
                blocks[idx + i] = BlockIdx::Emtpy(idx);
            }

            self.available_blocks.insert(idx);
        }

        blocks[block_id] = BlockIdx::OwnedStart(0);

        for i in 1..required_blocks {
            let owned_id = block_id + i;
            blocks[owned_id] = BlockIdx::Owned(block_id);
        }

        let internal = InternalBlockKey { idx: block_id, blocks: required_blocks };
        self.active_keys.insert(internal);

        BlockKey { idx: block_id, blocks: required_blocks, generation: self.generation }
    }
}


#[cfg(test)]
mod test {
    use std::sync::{
        atomic::{AtomicI32, Ordering},
        Arc,
    };
    use std::collections::BTreeSet;

    use super::BlockStorage;
    

    pub struct DropTest {
        value: Arc<AtomicI32>,
    }
    
    impl DropTest {
        pub fn new(value: Arc<AtomicI32>) -> Self {
            value.fetch_add(1, Ordering::SeqCst);
            println!("Created");
            Self { value }
        }
    }
    
    impl Drop for DropTest {
        fn drop(&mut self) {
            println!("Dropped");
            self.value.fetch_add(-1, Ordering::SeqCst);
        }
    }

    /// Simple test for testing that values get dropped correctly when clearing/dropping the storage
    #[test]
    fn drop_test() {
        let mut storage = BlockStorage::<DropTest>::new(10);
    
        let idx1 = storage.create(3);
        let idx2 = storage.create(15);
    
        println!("1: {:?}, 2: {:?}", idx1, idx2);
    
        let mut block1 = storage.get(idx1).unwrap();
        let mut block2 = storage.get(idx2).unwrap();
    
        let value = Arc::new(AtomicI32::new(0));
    
        block1.push(DropTest::new(value.clone()));
        block1.push(DropTest::new(value.clone()));
        block1.push(DropTest::new(value.clone()));
        block1.push(DropTest::new(value.clone()));
    
        block2.push(DropTest::new(value.clone()));
        block2.push(DropTest::new(value.clone()));
    
        block1.return_key();
        block2.return_key();
    
        assert!(value.load(Ordering::SeqCst) == 6);
        storage.clear();
        assert!(value.load(Ordering::SeqCst) == 0);

        let idx1 = storage.create(3);
        let idx2 = storage.create(15);

        let mut block1 = storage.get(idx1).unwrap();
        let mut block2 = storage.get(idx2).unwrap();
    
        block1.push(DropTest::new(value.clone()));
        block1.push(DropTest::new(value.clone()));
        block1.push(DropTest::new(value.clone()));
        block1.push(DropTest::new(value.clone()));
    
        block2.push(DropTest::new(value.clone()));
        block2.push(DropTest::new(value.clone()));
        assert!(value.load(Ordering::SeqCst) == 6);
        drop(storage);
        assert!(value.load(Ordering::SeqCst) == 0);
    }
    
    #[test]
    fn remove_isolated_test() {
        let mut storage = BlockStorage::<DropTest>::new(10);
        let idx1 = storage.create(10);
        let idx2 = storage.create(20);
        let idx3 = storage.create(10);
        let value = Arc::new(AtomicI32::new(0));
        
        let mut block1 = storage.get(idx1).unwrap();
        block1.push(DropTest::new(value.clone()));
        block1.push(DropTest::new(value.clone()));

        let mut block2 = storage.get(idx2).unwrap();
        block2.push(DropTest::new(value.clone()));
        block2.push(DropTest::new(value.clone()));
        block2.push(DropTest::new(value.clone()));

        let mut block3 = storage.get(idx3).unwrap();
        block3.push(DropTest::new(value.clone()));
        block3.push(DropTest::new(value.clone()));

        assert!(value.load(Ordering::SeqCst) == 7);

        let idx2 = block2.return_key();
        storage.remove(idx2);
        assert!(value.load(Ordering::SeqCst) == 4);

        let mut set = BTreeSet::new(); 
        set.insert(1);

        assert!(storage.available_blocks == set);
    }   

    #[test]
    fn remove_left_missing_test() {
        let mut storage = BlockStorage::<DropTest>::new(10);
        let idx1 = storage.create(10);
        let idx2 = storage.create(20);
        let idx3 = storage.create(10);
        let value = Arc::new(AtomicI32::new(0));
        
        let mut block1 = storage.get(idx1).unwrap();
        block1.push(DropTest::new(value.clone()));
        block1.push(DropTest::new(value.clone()));

        let mut block2 = storage.get(idx2).unwrap();
        block2.push(DropTest::new(value.clone()));
        block2.push(DropTest::new(value.clone()));
        block2.push(DropTest::new(value.clone()));

        let mut block3 = storage.get(idx3).unwrap();
        block3.push(DropTest::new(value.clone()));
        block3.push(DropTest::new(value.clone()));

        assert!(value.load(Ordering::SeqCst) == 7);
        let idx1 = block1.return_key();
        let idx2 = block2.return_key();
        storage.remove(idx1);
        assert!(value.load(Ordering::SeqCst) == 5);
        let mut set = BTreeSet::new(); 
        set.insert(0);
        assert!(storage.available_blocks == set);

        storage.remove(idx2);
        assert!(value.load(Ordering::SeqCst) == 2);

        let mut set = BTreeSet::new(); 
        set.insert(0);

        assert!(storage.available_blocks == set);

        // Check that the 0th block is one cohesive block that can be allocated entirely
        let idx4 = storage.create(30);
        assert!(idx4.idx == 0);
        assert!(idx4.blocks == 3);

        assert!(storage.available_blocks == BTreeSet::new());
    }   

    #[test]
    fn remove_right_missing_test() {
        let mut storage = BlockStorage::<DropTest>::new(10);
        let idx1 = storage.create(10);
        let idx2 = storage.create(20);
        let idx3 = storage.create(10);
        let value = Arc::new(AtomicI32::new(0));
        
        let mut block1 = storage.get(idx1).unwrap();
        block1.push(DropTest::new(value.clone()));
        block1.push(DropTest::new(value.clone()));

        let mut block2 = storage.get(idx2).unwrap();
        block2.push(DropTest::new(value.clone()));
        block2.push(DropTest::new(value.clone()));
        block2.push(DropTest::new(value.clone()));

        let mut block3 = storage.get(idx3).unwrap();
        block3.push(DropTest::new(value.clone()));
        block3.push(DropTest::new(value.clone()));

        assert!(value.load(Ordering::SeqCst) == 7);
        let idx1 = block1.return_key();
        let idx2 = block2.return_key();
        
        storage.remove(idx2);
        assert!(value.load(Ordering::SeqCst) == 4);
        let mut set = BTreeSet::new(); 
        set.insert(1);
        assert!(storage.available_blocks == set);
        
        storage.remove(idx1);
        assert!(value.load(Ordering::SeqCst) == 2);

        let mut set = BTreeSet::new(); 
        set.insert(0);

        assert!(storage.available_blocks == set);

        // Check that the 0th block is one cohesive block that can be allocated entirely
        let idx4 = storage.create(30);
        assert!(idx4.idx == 0);
        assert!(idx4.blocks == 3);

        assert!(storage.available_blocks == BTreeSet::new());
    }   

    #[test]
    fn remove_left_right_missing_test() {
        let mut storage = BlockStorage::<DropTest>::new(10);
        let idx1 = storage.create(10);
        let idx2 = storage.create(20);
        let idx3 = storage.create(10);
        let value = Arc::new(AtomicI32::new(0));
        
        let mut block1 = storage.get(idx1).unwrap();
        block1.push(DropTest::new(value.clone()));
        block1.push(DropTest::new(value.clone()));

        let mut block2 = storage.get(idx2).unwrap();
        block2.push(DropTest::new(value.clone()));
        block2.push(DropTest::new(value.clone()));
        block2.push(DropTest::new(value.clone()));

        let mut block3 = storage.get(idx3).unwrap();
        block3.push(DropTest::new(value.clone()));
        block3.push(DropTest::new(value.clone()));

        assert!(value.load(Ordering::SeqCst) == 7);
        let idx1 = block1.return_key();
        let idx2 = block2.return_key();
        let idx3 = block3.return_key();
        
        storage.remove(idx1);
        assert!(value.load(Ordering::SeqCst) == 5);
        let mut set = BTreeSet::new(); 
        set.insert(0);
        assert!(storage.available_blocks == set);

        storage.remove(idx3);
        assert!(value.load(Ordering::SeqCst) == 3);
        let mut set = BTreeSet::new(); 
        set.insert(0);
        set.insert(3);
        assert!(storage.available_blocks == set);
        
        storage.remove(idx2);
        assert!(value.load(Ordering::SeqCst) == 0);

        let mut set = BTreeSet::new(); 
        set.insert(0);

        assert!(storage.available_blocks == set);

        let idx4 = storage.create(40);
        assert!(idx4.idx == 0);
        assert!(idx4.blocks == 4);
        assert!(storage.available_blocks == BTreeSet::new());
    }   

    #[test]
    fn remove_end_test() {
        let mut storage = BlockStorage::<DropTest>::new(10);
        let idx1 = storage.create(10);
        let idx2 = storage.create(20);
        let idx3 = storage.create(10);
        let value = Arc::new(AtomicI32::new(0));
        
        let mut block1 = storage.get(idx1).unwrap();
        block1.push(DropTest::new(value.clone()));
        block1.push(DropTest::new(value.clone()));

        let mut block2 = storage.get(idx2).unwrap();
        block2.push(DropTest::new(value.clone()));
        block2.push(DropTest::new(value.clone()));
        block2.push(DropTest::new(value.clone()));

        let mut block3 = storage.get(idx3).unwrap();
        block3.push(DropTest::new(value.clone()));
        block3.push(DropTest::new(value.clone()));

        assert!(value.load(Ordering::SeqCst) == 7);
        let idx3 = block3.return_key();
        
        storage.remove(idx3);
        assert!(value.load(Ordering::SeqCst) == 5);
        let mut set = BTreeSet::new(); 
        set.insert(3);
        assert!(storage.available_blocks == set);

        // Check that the 0th block is one cohesive block that can be allocated entirely
        let idx4 = storage.create(20);
        assert!(idx4.idx == 3);
        assert!(idx4.blocks == 2);

        assert!(storage.available_blocks == BTreeSet::new());
    }  
}