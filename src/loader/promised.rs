use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, Weak,
};

#[derive(Debug)]
pub struct OneTimeLock<T> {
    written: bool,
    ready: Arc<AtomicBool>,
    data: Weak<Mutex<Option<T>>>,
}

impl<T> OneTimeLock<T> {
    pub fn write(&mut self, value: T) -> bool {
        if self.ready.load(Ordering::Relaxed) {
            return false;
        }

        let data = match self.data.upgrade() {
            Some(data) => data,
            None => return false,
        };

        *data.lock().unwrap() = Some(value);
        self.written = true;
        true
    }

    pub fn unlock(&self) -> bool {
        if !self.written {
            return false;
        }

        self.ready.store(true, Ordering::Relaxed);
        true
    }
}

#[derive(Debug)]
pub struct OneTimeMutex<T> {
    locked: bool,
    ready: Arc<AtomicBool>,
    data: Arc<Mutex<Option<T>>>,
}

impl<T> OneTimeMutex<T> {
    pub fn new() -> Self {
        Self {
            locked: false,
            ready: Arc::new(AtomicBool::new(false)),
            data: Arc::new(Mutex::new(None)),
        }
    }

    pub fn lock(&mut self) -> Option<OneTimeLock<T>> {
        if self.locked {
            return None;
        }
        
        self.locked = true;
        let data = Arc::downgrade(&self.data);
        let ready = self.ready.clone();

        Some(OneTimeLock {
            ready,
            data,
            written: false,
        })
    }

    pub fn is_ready(&self) -> bool {
        self.ready.load(Ordering::Relaxed)
    }

    pub fn into_ready(mut self) -> Result<T, OneTimeMutex<T>> {
        if self.is_ready() {
            let value = match Arc::try_unwrap(self.data) {
                Ok(value) => value.into_inner().unwrap().unwrap(),
                Err(data) => {
                    self.data = data;
                    return Err(self);
                }
            };

            return Ok(value);
        }

        Err(self)
    }
}

pub enum PromisedValue<T> {
    Loaded(T),
    Loading(OneTimeMutex<T>),
}

impl<T> PromisedValue<T> {
    pub fn new_loading() -> (Self, OneTimeLock<T>) {
        let mut value = OneTimeMutex::new();
        let lock = value.lock().unwrap();
        (Self::Loading(value), lock)
    }

    pub fn unwrap_loaded(self) -> T {
        match self {
            PromisedValue::Loaded(value) => value,
            _ => panic!("Tried to unwrap not loaded value as loaded"),
        }
    }

    pub fn unwrap_loading(self) -> OneTimeMutex<T> {
        match self {
            PromisedValue::Loading(value) => value,
            _ => panic!("Tried to unwrap loaded value as loading"),
        }
    }

    pub fn get(&self) -> Option<&T> {
        match self {
            Self::Loaded(value) => Some(value),
            _ => None
        }
    }

    pub fn update_get(&mut self) -> Option<&T> {
        if let PromisedValue::Loaded(value) = self {
            return Some(value);
        }

        take_mut::take(self, |value| {
            let one_time = value.unwrap_loading();

            match one_time.into_ready() {
                Ok(value) => PromisedValue::Loaded(value),
                Err(one_time) => PromisedValue::Loading(one_time),
            }
        });

        if let PromisedValue::Loaded(value) = self {
            return Some(value);
        }

        None
    }
}
