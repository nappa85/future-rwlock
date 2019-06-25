# Future-RwLock

This is an "as simple as possible" Future implementation for [parking_lot::RwLock](https://docs.rs/parking_lot/0.8.0/parking_lot/type.RwLock.html).
It works on `AsRef<RwLock<T>>` instead of directly on `RwLock<T>` for borrowing reasons (RwLock must not live inside the Future, it would be dropped with it).

Example:
```
use std::sync::Arc;

use tokio::run;

use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use future_rwlock::{FutureReadable, FutureWriteable};

use lazy_static::lazy_static;

lazy_static! {
    static ref LOCK: Arc<RwLock<Vec<String>>> = Arc::new(RwLock::new(Vec::new()));
}

fn main() {
    run(LOCK.future_write(|mut v: RwLockWriteGuard<'_, Vec<String>>| {
        v.push(String::from("It works!"));
        LOCK.future_read(|v: RwLockReadGuard<'_, Vec<String>>| -> Result<(), ()> {
            assert!(v.len() == 1 && v[0] == "It works!");
            Ok(())
        })
    }));
}
```
