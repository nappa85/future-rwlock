use std::sync::Arc;

use tokio::prelude::{Async, future::{Future, IntoFuture}};

use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};

pub struct FutureRead<F, T, I>
where
    F: FnMut(RwLockReadGuard<'_, T>) -> I,
    I: IntoFuture,
{
    lock: Arc<RwLock<T>>,
    func: F,
}

impl<F, T, I> FutureRead<F, T, I>
where
    F: FnMut(RwLockReadGuard<'_, T>) -> I,
    I: IntoFuture,
{
    fn new(lock: Arc<RwLock<T>>, func: F) -> Self {
        FutureRead {
            lock,
            func,
        }
    }
}

impl<F, T, I> Future for FutureRead<F, T, I>
where
    F: FnMut(RwLockReadGuard<'_, T>) -> I,
    I: IntoFuture,
{
    type Item = <<I as IntoFuture>::Future as Future>::Item;
    type Error = <<I as IntoFuture>::Future as Future>::Error;

    fn poll(&mut self) -> Result<Async<Self::Item>, Self::Error> {
        match self.lock.try_read() {
            Some(read_lock) => (self.func)(read_lock).into_future().poll(),
            None => Ok(Async::NotReady),
        }
    }
}

pub trait FutureReadable<T, I: IntoFuture> {
    fn future_read<F: FnMut(RwLockReadGuard<'_, T>) -> I>(&self, func: F) -> FutureRead<F, T, I>;
}

impl<T, I: IntoFuture> FutureReadable<T, I> for Arc<RwLock<T>> {
    fn future_read<F: FnMut(RwLockReadGuard<'_, T>) -> I>(&self, func: F) -> FutureRead<F, T, I> {
        FutureRead::new(Arc::clone(self), func)
    }
}

pub struct FutureWrite<F, T, I>
where
    F: FnMut(RwLockWriteGuard<'_, T>) -> I,
    I: IntoFuture,
{
    lock: Arc<RwLock<T>>,
    func: F,
}

impl<F, T, I> FutureWrite<F, T, I>
where
    F: FnMut(RwLockWriteGuard<'_, T>) -> I,
    I: IntoFuture,
{
    fn new(lock: Arc<RwLock<T>>, func: F) -> Self {
        FutureWrite {
            lock,
            func,
        }
    }
}

impl<F, T, I> Future for FutureWrite<F, T, I>
where
    F: FnMut(RwLockWriteGuard<'_, T>) -> I,
    I: IntoFuture,
{
    type Item = <<I as IntoFuture>::Future as Future>::Item;
    type Error = <<I as IntoFuture>::Future as Future>::Error;

    fn poll(&mut self) -> Result<Async<Self::Item>, Self::Error> {
        match self.lock.try_write() {
            Some(write_lock) => (self.func)(write_lock).into_future().poll(),
            None => Ok(Async::NotReady),
        }
    }
}

pub trait FutureWriteable<T, I: IntoFuture> {
    fn future_write<F: FnMut(RwLockWriteGuard<'_, T>) -> I>(&self, func: F) -> FutureWrite<F, T, I>;
}

impl<T, I: IntoFuture> FutureWriteable<T, I> for Arc<RwLock<T>> {
    fn future_write<F: FnMut(RwLockWriteGuard<'_, T>) -> I>(&self, func: F) -> FutureWrite<F, T, I> {
        FutureWrite::new(Arc::clone(self), func)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use tokio::runtime::current_thread;

    use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};

    use super::{FutureReadable, FutureWriteable};

    use lazy_static::lazy_static;

    lazy_static! {
        static ref LOCK: Arc<RwLock<Vec<String>>> = Arc::new(RwLock::new(Vec::new()));
    }

    #[test]
    fn write_and_read() {
        current_thread::block_on_all(LOCK.future_write(|mut v: RwLockWriteGuard<'_, Vec<String>>| {
            &v.push(String::from("It works!"));
            LOCK.future_read(|v: RwLockReadGuard<'_, Vec<String>>| {
                if v.len() == 1 && v[0] == "It works!" {
                    Ok(())
                }
                else {
                    Err(())
                }
            })
        })).unwrap();
    }
}
