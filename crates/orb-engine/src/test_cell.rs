//! The unit tests' shared logs. The test devices log into a cell they share with the test, and
//! [`crate::frame::SongDevice`] is `Send` (task-043), so the cell is a `Mutex` behind an `Arc`
//! with `RefCell`'s method names: the tests keep their `Rc<RefCell<_>>` spelling.

use std::sync::{Mutex, MutexGuard, TryLockError};

/// `std::rc::Rc`'s place: an `Arc`.
pub type Rc<T> = std::sync::Arc<T>;

/// `std::cell::RefCell`'s place: a `Mutex` (a poisoned lock is taken over, as a failed test
/// has already reported). The tests use each cell on one thread, so a held lock is a double
/// borrow: it panics, as `RefCell` does, instead of deadlocking.
#[derive(Debug, Default)]
pub struct RefCell<T>(Mutex<T>);

impl<T> RefCell<T> {
    pub fn new(x: T) -> Self {
        RefCell(Mutex::new(x))
    }

    pub fn borrow(&self) -> MutexGuard<'_, T> {
        match self.0.try_lock() {
            Ok(g) => g,
            Err(TryLockError::Poisoned(e)) => e.into_inner(),
            Err(TryLockError::WouldBlock) => panic!("already borrowed"),
        }
    }

    pub fn borrow_mut(&self) -> MutexGuard<'_, T> {
        self.borrow()
    }
}
