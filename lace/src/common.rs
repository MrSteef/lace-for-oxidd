// deque backing buffer size, increase when encountering task stack overflows
#[cfg(not(feature = "chase_lev"))]
pub(crate) const BUFFER_SIZE: usize = 100000;

// task inlining threshold
#[cfg(feature = "unsafe_task")]
pub(crate) const TASK_INLINE_SIZE: usize = 6 * std::mem::size_of::<usize>();

// sentinel values for the thief flag on tasks
pub(crate) const THIEF_NONE: usize = usize::MAX;
pub(crate) const THIEF_FINISHED: usize = usize::MAX - 1;

// the type of the worker function
use crate::Worker;
pub(crate) type Function<I, O> = fn(&mut Worker, I) -> O;

// debugging utilities
use std::sync::atomic::AtomicUsize;
#[allow(dead_code)]
pub static mut LOG_SEQ: AtomicUsize = AtomicUsize::new(0);
#[macro_export]
macro_rules! log {
    ($($msg:tt),*) => {{
        use std::sync::atomic::Ordering;
        #[allow(unused_unsafe)]
        let s = unsafe { crate::common::LOG_SEQ.fetch_add(1, Ordering::SeqCst) };
        let me = std::thread::current();
        eprintln!("[{:?} {s}] {}", me.id(), format!($($msg),*));
    }}
}
#[macro_export]
macro_rules! muteable_assert {
    ($cond:expr, $rest:block) => {
        #[cfg(feature = "extra_assertions")]
        if !($cond) {
            $rest
            panic!("Assertion Failed");
        }
    }
}
