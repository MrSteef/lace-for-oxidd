use crate::common::THIEF_NONE;
use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::sync::atomic::AtomicUsize;

pub struct DequeBuffer<T> {
    pub size: usize,
    tasks: Box<[UnsafeCell<MaybeUninit<T>>]>,
    pub thief: Box<[AtomicUsize]>,
}
impl<T> DequeBuffer<T> {
    pub fn new(size: usize) -> Self {
        Self {
            size,
            tasks: Vec::into_boxed_slice(
                vec![(); size]
                    .into_iter()
                    .map(|_| UnsafeCell::new(MaybeUninit::uninit()))
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
        unsafe { std::ptr::write(self.ptr(i), task) }
    }
    pub fn take(&self, i: usize) -> T {
        unsafe { std::ptr::read(self.ptr(i)) }
    }
    pub fn ptr(&self, i: usize) -> *mut T {
        self.tasks.as_ptr().wrapping_add(i) as *mut T
    }
}

unsafe impl<T> Sync for DequeBuffer<T> {}
