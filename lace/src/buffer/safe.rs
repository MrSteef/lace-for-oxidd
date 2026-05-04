use crate::common::THIEF_NONE;
use crate::muteable_assert;
use std::cell::Cell;
use std::sync::atomic::AtomicUsize;

pub struct DequeBuffer<T> {
    pub size: usize,
    tasks: Box<[Cell<Option<T>>]>,
    pub thief: Box<[AtomicUsize]>,
}
impl<T> DequeBuffer<T> {
    pub fn new(size: usize) -> Self {
        Self {
            size,
            tasks: Vec::into_boxed_slice(
                vec![(); size]
                    .into_iter()
                    .map(|_| Cell::new(None))
                    .collect(),
            ),
            thief: Vec::into_boxed_slice(
                vec![(); size]
                    .into_iter()
                    .map(|_| AtomicUsize::new(THIEF_NONE))
                    .collect(),
            ),
        }
    }
    pub fn put(&self, i: usize, task: T) {
        let _old = self.tasks[i].replace(Some(task));
        muteable_assert!(_old.is_none(), {
            eprintln!("Invalid Deque State: put() at non-empty slot {}", i);
        });
    }
    pub fn take(&self, i: usize) -> T {
        let old = self.tasks[i].replace(None);
        muteable_assert!(old.is_some(), {
            eprintln!("Invalid Deque State: take() from empty slot {}", i);
        });
        old.unwrap()
    }
    #[cfg(feature = "unsafe_steal")]
    pub fn ptr(&self, i: usize) -> *mut T {
        let tptr = self.tasks[i].as_ptr();
        unsafe {
            match *tptr {
                None => panic!("Invalid Deque State: ptr() to empty slot {}", i),
                Some(ref mut t) => t,
            }
        }
    }
}

unsafe impl<T> Sync for DequeBuffer<T> {}
