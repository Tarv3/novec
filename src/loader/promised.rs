use cbc::{bounded, Receiver, Sender};
use std::{error::Error, fmt::{self, Display}};

use super::Convert;

#[derive(Debug)]
pub enum PromiseError<E> {
    Disconnected,
    LoadError(E),
}

impl<E: Error> Display for PromiseError<E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Disconnected => write!(f, "Promise receiver disconnected"),
            Self::LoadError(error) => write!(f, "Failed to load: {}", error),
        }
    }
}

impl<E: Error> Error for PromiseError<E> {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UpdateStatus {
    AlreadyOwned,
    Updated,
    Waiting,
}

#[derive(Debug)]
pub struct PromiseSender<T, M> {
    sender: Sender<T>,
    pub meta_data: M,
}

impl<T, M> PromiseSender<T, M> {
    pub fn send(&self, value: T) -> Result<(), cbc::TrySendError<T>> {
        self.sender.try_send(value)
    }
}

#[derive(Debug)]
pub enum Promise<T, U> {
    Owned(T),
    Waiting(Receiver<U>),
}

impl<T, U> Promise<T, U> {
    pub fn new_waiting<M>(meta: M) -> (Self, PromiseSender<U, M>) {
        let (sender, receiver) = bounded(1);
        let promise_sender = PromiseSender {
            sender,
            meta_data: meta,
        };
        (Self::Waiting(receiver), promise_sender)
    }

    pub fn get(&self) -> Option<&T> {
        match self {
            Self::Owned(value) => Some(value),
            _ => None,
        }
    }

    pub fn unwrap_waiting(self) -> Receiver<U> {
        match self {
            Self::Waiting(rec) => rec,
            _ => panic!("Tried to unwrap owned value"),
        }
    }

    pub fn unwrap(self) -> T {
        match self {
            Self::Owned(value) => value,
            _ => panic!("Tried to unwrap unfulfilled promise"),
        }
    }

    pub fn unwrap_ref(&self) -> &T {
        match self {
            Self::Owned(value) => value,
            _ => panic!("Tried to unwrap unfulfilled promise"),
        }
    }

    pub fn is_owned(&self) -> bool {
        match self {
            Promise::Owned(_) => true,
            Promise::Waiting(_) => false,
        }
    }
}

impl<T, U> Promise<T, U>
where
    U: Convert<T>,
{
    pub fn update(&mut self) -> Result<UpdateStatus, PromiseError<U::Error>> {
        match self {
            Self::Owned(_) => return Ok(UpdateStatus::AlreadyOwned),
            _ => (),
        }

        let mut result = Ok(UpdateStatus::Waiting);

        take_mut::take(self, |value| {
            let receiver = value.unwrap_waiting();

            match receiver.try_recv() {
                Ok(value) => match value.convert() {
                    Ok(owned) => {
                        result = Ok(UpdateStatus::Updated);
                        return Promise::Owned(owned);
                    }
                    Err(e) => {
                        result = Err(PromiseError::LoadError(e));
                        return Promise::Waiting(receiver);
                    }
                },
                Err(cbc::TryRecvError::Disconnected) => {
                    result = Err(PromiseError::Disconnected);
                    return Promise::Waiting(receiver);
                }
                _ => return Promise::Waiting(receiver),
            }
        });

        result
    }

    pub fn update_blocking(&mut self) -> Result<UpdateStatus, PromiseError<U::Error>>
    {
        let value = match self {
            Self::Owned(_) => return Ok(UpdateStatus::AlreadyOwned),
            Self::Waiting(receiver) => receiver
                .recv()
                .or_else(|_| Err(PromiseError::Disconnected))?,
        };

        let owned = match value.convert() {
            Ok(success) => success,
            Err(e) => return Err(PromiseError::LoadError(e)),
        };

        *self = Self::Owned(owned);
        Ok(UpdateStatus::Updated)
    }
}
