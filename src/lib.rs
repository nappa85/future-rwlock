#![deny(warnings)]
#![deny(missing_docs)]

//! # future-rwlock
//!
//! A simple Future implementation for parking_lot/RwLock

use std::convert::AsRef;
use std::marker::PhantomData;

use tokio::prelude::{Async, future::{Future, IntoFuture}};

use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};

/// Wrapper to read from RwLock in Future-style
pub struct FutureRead<'a, R, T, F, I>
where
    R: AsRef<RwLock<T>>,
    F: FnMut(RwLockReadGuard<'_, T>) -> I,
    I: IntoFuture,
{
    lock: &'a R,
    func: F,
    _contents: PhantomData<T>,
    _future: PhantomData<I>,
}

impl<'a, R, T, F, I> FutureRead<'a, R, T, F, I>
where
    R: AsRef<RwLock<T>>,
    F: FnMut(RwLockReadGuard<'_, T>) -> I,
    I: IntoFuture,
{
    fn new(lock: &'a R, func: F) -> Self {
        FutureRead {
            lock,
            func,
            _contents: PhantomData,
            _future: PhantomData,
        }
    }
}

impl<'a, R, T, F, I> Future for FutureRead<'a, R, T, F, I>
where
    R: AsRef<RwLock<T>>,
    F: FnMut(RwLockReadGuard<'_, T>) -> I,
    I: IntoFuture,
{
    type Item = <<I as IntoFuture>::Future as Future>::Item;
    type Error = <<I as IntoFuture>::Future as Future>::Error;

    fn poll(&mut self) -> Result<Async<Self::Item>, Self::Error> {
        match self.lock.as_ref().try_read() {
            Some(read_lock) => (self.func)(read_lock).into_future().poll(),
            None => Ok(Async::NotReady),
        }
    }
}

/// Trait to permit FutureRead implementation on wrapped RwLock (not RwLock itself)
pub trait FutureReadable<R: AsRef<RwLock<T>>, T, I: IntoFuture> {
    /// Takes a closure that will be executed when the Futures gains the read-lock
    fn future_read<F: FnMut(RwLockReadGuard<'_, T>) -> I>(&self, func: F) -> FutureRead<R, T, F, I>;
}

impl<R: AsRef<RwLock<T>>, T, I: IntoFuture> FutureReadable<R, T, I> for R {
    fn future_read<F: FnMut(RwLockReadGuard<'_, T>) -> I>(&self, func: F) -> FutureRead<R, T, F, I> {
        FutureRead::new(self, func)
    }
}

/// Wrapper to write into RwLock in Future-style
pub struct FutureWrite<'a, R, T, F, I>
where
    R: AsRef<RwLock<T>>,
    F: FnMut(RwLockWriteGuard<'_, T>) -> I,
    I: IntoFuture,
{
    lock: &'a R,
    func: F,
    _contents: PhantomData<T>,
    _future: PhantomData<I>,
}

impl<'a, R, T, F, I> FutureWrite<'a, R, T, F, I>
where
    R: AsRef<RwLock<T>>,
    F: FnMut(RwLockWriteGuard<'_, T>) -> I,
    I: IntoFuture,
{
    fn new(lock: &'a R, func: F) -> Self {
        FutureWrite {
            lock,
            func,
            _contents: PhantomData,
            _future: PhantomData,
        }
    }
}

impl<'a, R, T, F, I> Future for FutureWrite<'a, R, T, F, I>
where
    R: AsRef<RwLock<T>>,
    F: FnMut(RwLockWriteGuard<'_, T>) -> I,
    I: IntoFuture,
{
    type Item = <<I as IntoFuture>::Future as Future>::Item;
    type Error = <<I as IntoFuture>::Future as Future>::Error;

    fn poll(&mut self) -> Result<Async<Self::Item>, Self::Error> {
        match self.lock.as_ref().try_write() {
            Some(write_lock) => (self.func)(write_lock).into_future().poll(),
            None => Ok(Async::NotReady),
        }
    }
}

/// Trait to permit FutureWrite implementation on wrapped RwLock (not RwLock itself)
pub trait FutureWriteable<R: AsRef<RwLock<T>>, T, I: IntoFuture> {
    /// Takes a closure that will be executed when the Futures gains the write-lock
    fn future_write<F: FnMut(RwLockWriteGuard<'_, T>) -> I>(&self, func: F) -> FutureWrite<R, T, F, I>;
}

impl<R: AsRef<RwLock<T>>, T, I: IntoFuture> FutureWriteable<R, T, I> for R {
    fn future_write<F: FnMut(RwLockWriteGuard<'_, T>) -> I>(&self, func: F) -> FutureWrite<R, T, F, I> {
        FutureWrite::new(self, func)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::rc::Rc;

    use tokio::runtime::current_thread;

    use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};

    use super::{FutureReadable, FutureWriteable};

    use lazy_static::lazy_static;

    lazy_static! {
        static ref LOCK: Arc<RwLock<Vec<String>>> = Arc::new(RwLock::new(Vec::new()));
    }

    #[test]
    fn current_thread_lazy_static() {
        current_thread::block_on_all(LOCK.future_write(|mut v: RwLockWriteGuard<'_, Vec<String>>| {
            v.push(String::from("It works!"));
            LOCK.future_read(|v: RwLockReadGuard<'_, Vec<String>>| -> Result<(), ()> {
                // since we are using the same lazy_static as multithread_lazy_static
                assert!((v.len() == 1 && v[0] == "It works!") || (v.len() == 2 && v[0] == "It works!" && v[1] == "It works!"));
                Ok(())
            })
        })).unwrap();
    }

    #[test]
    fn current_thread_local_arc() {
        let lock = Arc::new(RwLock::new(Vec::new()));
        current_thread::block_on_all(lock.future_write(|mut v: RwLockWriteGuard<'_, Vec<String>>| {
            v.push(String::from("It works!"));
            lock.future_read(|v: RwLockReadGuard<'_, Vec<String>>| -> Result<(), ()> {
                assert!(v.len() == 1 && v[0] == "It works!");
                Ok(())
            })
        })).unwrap();
    }

    #[test]
    fn current_thread_local_rc() {
        let lock = Rc::new(RwLock::new(Vec::new()));
        current_thread::block_on_all(lock.future_write(|mut v: RwLockWriteGuard<'_, Vec<String>>| {
            v.push(String::from("It works!"));
            lock.future_read(|v: RwLockReadGuard<'_, Vec<String>>| -> Result<(), ()> {
                assert!(v.len() == 1 && v[0] == "It works!");
                Ok(())
            })
        })).unwrap();
    }

    #[test]
    fn current_thread_local_box() {
        let lock = Box::new(RwLock::new(Vec::new()));
        current_thread::block_on_all(lock.future_write(|mut v: RwLockWriteGuard<'_, Vec<String>>| {
            v.push(String::from("It works!"));
            lock.future_read(|v: RwLockReadGuard<'_, Vec<String>>| -> Result<(), ()> {
                assert!(v.len() == 1 && v[0] == "It works!");
                Ok(())
            })
        })).unwrap();
    }

    #[test]
    fn multithread_lazy_static() {
        tokio::run(LOCK.future_write(|mut v: RwLockWriteGuard<'_, Vec<String>>| {
            v.push(String::from("It works!"));
            LOCK.future_read(|v: RwLockReadGuard<'_, Vec<String>>| -> Result<(), ()> {
                // Since we are using the same lazy_static as current_thread_lazy_static
                assert!((v.len() == 1 && v[0] == "It works!") || (v.len() == 2 && v[0] == "It works!" && v[1] == "It works!"));
                Ok(())
            })
        }));
    }

    // Implies a lifetime problem
    // #[test]
    // fn multithread_local_arc() {
    //     let lock = Arc::new(RwLock::new(Vec::new()));
    //     tokio::run(lock.future_write(|mut v: RwLockWriteGuard<'_, Vec<String>>| {
    //         &v.push(String::from("It works!"));
    //         lock.future_read(|v: RwLockReadGuard<'_, Vec<String>>| -> Result<(), ()> {
    //             assert!(v.len() == 1 && v[0] == "It works!");
    //             Ok(())
    //         })
    //     }));
    // }

    // Can't be done because Rc isn't Sync
    // #[test]
    // fn multithread_local_rc() {
    //     let lock = Rc::new(RwLock::new(Vec::new()));
    //     tokio::run(lock.future_write(|mut v: RwLockWriteGuard<'_, Vec<String>>| {
    //         &v.push(String::from("It works!"));
    //         lock.future_read(|v: RwLockReadGuard<'_, Vec<String>>| -> Result<(), ()> {
    //             assert!(v.len() == 1 && v[0] == "It works!");
    //             Ok(())
    //         })
    //     }));
    // }

    // Implies a lifetime problem
    // #[test]
    // fn multithread_local_box() {
    //     let lock = Box::new(RwLock::new(Vec::new()));
    //     tokio::run(lock.future_write(|mut v: RwLockWriteGuard<'_, Vec<String>>| {
    //         &v.push(String::from("It works!"));
    //         lock.future_read(|v: RwLockReadGuard<'_, Vec<String>>| -> Result<(), ()> {
    //             assert!(v.len() == 1 && v[0] == "It works!");
    //             Ok(())
    //         })
    //     }));
    // }
}
