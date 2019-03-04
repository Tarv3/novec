use take_mut::take;

// None points to the next closest empty entry;
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Entry<T> {
    Data(T),
    Next(usize)
}

impl<T> Entry<T> {
    pub fn is_next(&self) -> bool {
        match self {
            Entry::Next(_) => true,
            _ => false
        }
    }

    pub fn is_data(&self) -> bool {
        match self {
            Entry::Data(_) => true,
            _ => false
        }
    }

    pub fn next_ref_mut(&mut self) -> &mut usize {
        match self {
            Entry::Next(ref mut next) => next,
            _ => panic!("Tried to unwrap data")
        }
    }

    pub fn data_ref(&self) -> &T {
        match &self {
            Entry::Data(data) => data,
            _ => panic!("Tried to unwrap next")
        }
    }

    pub fn data_ref_mut(&mut self) -> &mut T {
        match self {
            Entry::Data(ref mut data) => data,
            _ => panic!("Tried to unwrap next")
        }
    }

    pub fn unwrap_next(&self) -> usize {
        match self {
            Entry::Next(next) => *next,
            _ => panic!("Tried to unwrap data")
        }
    }

    pub fn unwrap_data(self) -> T {
        match self {
            Entry::Data(data) => data,
            _ => panic!("Tried to unwrap next")
        }
    }

    pub fn swap_next(&mut self, next: usize) -> Option<T> {
        let mut value = None;
        take(self, |x| {
            match x {
                Entry::Data(data) => {
                    value = Some(data);
                    Entry::Next(next)
                }
                _ => Entry::Next(next)
            }
        });

        value
    }
    
    pub fn swap_data(&mut self, input: T) -> Option<T> {
        let mut value = None;
        take(self, |x| {
            match x {
                Entry::Data(data) => {
                    value = Some(data);
                    Entry::Data(input)
                }
                _ => Entry::Data(input)
            }
        });

        value
    }

    pub fn insert_data(&mut self, data: T) {
        *self = Entry::Data(data)
    }

    pub fn set_next(&mut self, next: usize) {
        *self = Entry::Next(next)
    }

    pub fn option(self) -> Option<T> {
        match self {
            Entry::Data(data) => Some(data),
            Entry::Next(_) => None,
        }
    }

    pub fn option_ref(&self) -> Option<&T> {
        match self {
            Entry::Data(ref data) => Some(data),
            Entry::Next(_) => None,
        }
    }

    pub fn option_ref_mut(&mut self) -> Option<&mut T> {
        match self {
            Entry::Data(ref mut data) => Some(data),
            Entry::Next(_) => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct NoVec<T> {
    next: usize,
    entries: Vec<Entry<T>>,
}

impl<T> NoVec<T> {
    pub fn with_capacity(cap: usize) -> NoVec<T> {
        let entries = Vec::with_capacity(cap);

        NoVec {
            next: 0,
            entries
        }
    }

    pub fn new() -> NoVec<T> {
        NoVec {
            next: 0,
            entries: vec![]
        }
    }

    pub fn next_id(&self) -> usize {
        self.next
    }

    pub fn entry(&self, index: usize) -> Option<&T> {
        if index >= self.entries.len() {
            return None;
        }

        self.entries[index].option_ref()
    }

    pub fn entry_mut(&mut self, index: usize) -> Option<&mut T> {
        if index >= self.entries.len() {
            return None;
        }

        self.entries[index].option_ref_mut()
    }

    pub fn push(&mut self, value: T) -> usize {
        let output = self.next;
        if self.next >= self.entries.len() {
            self.entries.push(Entry::Data(value));
            self.next += 1;
        }
        else {
            let entry = &mut self.entries[self.next];
            let next = entry.unwrap_next();
            entry.insert_data(value);
            self.next = next;
        }

        output
    }
    
    pub fn entries_iter(&self) -> impl Iterator<Item = (usize, Option<&T>)> {
        self.entries.iter().enumerate().map(|(index, value)| (index, value.option_ref()))
    }
    
    pub fn entries_iter_mut(&mut self) -> impl Iterator<Item = (usize, Option<&mut T>)> {
        self.entries.iter_mut().enumerate().map(|(index, value)| (index, value.option_ref_mut()))
    }
    
    pub fn id_iter(&self) -> impl Iterator<Item = (usize, &T)> {
        self.entries.iter().enumerate().filter(|(_, x)| x.is_data()).map(|(index, x)| (index, x.data_ref()))
    }

    pub fn id_iter_mut(&mut self) -> impl Iterator<Item = (usize, &mut T)> {
        self.entries.iter_mut().enumerate().filter(|(_, x)| x.is_data()).map(|(index, x)| (index, x.data_ref_mut()))
    }

    pub fn values_iter(&self) -> impl Iterator<Item = &T> {
        self.entries.iter().filter(|x| x.is_data()).map(|x| x.data_ref())
    }

    pub fn values_iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.entries.iter_mut().filter(|x| x.is_data()).map(|x| x.data_ref_mut())
    }

    pub fn fill_to(&mut self, size: usize) {
        let len = self.entries.len();
        if len >= size {
            return;
        }

        for i in len..size {
            self.entries.push(Entry::Next(i + 1));
        }
    }
    
    pub fn remove(&mut self, index: usize) -> Option<T> {
        if index >= self.entries.len() {
            return None;
        }

        if self.entries[index].is_next() {
            return None;
        }

        if index < self.next {
            let value = self.entries[index].swap_next(self.next);
            self.next = index;

            return value;
        }

        let mut next = self.next;
        let mut prev_val = next;

        while next <= index {
            prev_val = next;
            next = self.entries[next].unwrap_next();
        }

        let value = self.entries[index].swap_next(next);
        self.entries[prev_val].set_next(index);

        value
    }
}

#[test]
fn it_works() {
    let mut vec = NoVec::with_capacity(5);
    vec.push(0);
    vec.push(1);
    vec.push(2);
    vec.push(3);
    vec.push(4);
    let output = vec.remove(1);
    println!("{:?}", vec);
    assert!(Some(1) == output);

    let output = vec.remove(3);
    println!("{:?}", vec);
    assert!(vec.next == 1);
    assert!(vec.entries[1] == Entry::Next(3));

    let pos = vec.push(1);
    println!("{:?}", vec);
    assert!(vec.next == 3);
    assert!(pos == 1);
    assert!(vec.entries[1] == Entry::Data(1));
    assert!(vec.entries[3] == Entry::Next(5));
}
