use crate::buffer::picked::DequeBuffer;
use crate::common::{THIEF_FINISHED, THIEF_NONE};
#[cfg(feature = "metrics")]
use crate::metrics::Metrics;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

extern crate crossbeam_utils;
use crossbeam_utils::CachePadded;

// logging/assertion macros for help when debugging
#[allow(unused_imports)]
use crate::{log, muteable_assert};
#[allow(unused_macros)]
macro_rules! qlog {
    ($self:expr, $($msg:tt),*) => {{
        let st = ($self).shared.split_tail.load(Ordering::Relaxed);
        log!("[t:{},s:{},h:{}]: {}", (st as u32 as usize), ((st >> 32) as u32 as usize), (($self).owned.head), (format!($($msg),*)))
    }}
}
macro_rules! qassert {
    ($cond:expr, $self:expr, $($msg:tt),*) => {
        muteable_assert!($cond, {
            qlog!($self, $($msg),*);
        })
    };
}
#[allow(unused_macros)]
macro_rules! owner_invariants {
    ($self:expr, $ctx:expr) => {
        let st = ($self).shared.split_tail.load(Ordering::Relaxed);
        let _t = st as u32 as usize;
        let _s = (st >> 32) as u32 as usize;
        qassert!(
            ($self).shared.allstolen.load(Ordering::Relaxed) == ($self).owned.allstolen,
            $self,
            "allstolen != o_allstolen @ {}",
            $ctx
        );
        if (!($self).owned.allstolen) {
            qassert!(
                ($self).owned.split == _s,
                $self,
                "o_split != split @ {}",
                $ctx
            );
            qassert!(_t <= ($self).owned.split, $self, "tail > split @ {}", $ctx);
            qassert!(
                ($self).owned.split <= ($self).owned.head,
                $self,
                "split > head @ {}",
                $ctx
            );
        }
    };
}

#[derive(PartialEq, Eq, Debug)]
pub enum Pop<T> {
    Work(T),       // work item
    Stolen(usize), // index of stolen task
}

struct DequeOwned {
    // private state
    head: usize,
    split: usize,
    allstolen: bool,
    #[cfg(feature = "metrics")]
    metrics: Arc<Metrics>,
}
struct DequeShared<T> {
    // shared state
    split_tail: AtomicUsize,
    splitreq: AtomicBool,
    allstolen: AtomicBool,
    buffer: DequeBuffer<T>,
    #[cfg(feature = "metrics")]
    metrics: Arc<Metrics>,
}

impl DequeOwned {
    fn new(#[cfg(feature = "metrics")] metrics: Arc<Metrics>) -> Self {
        Self {
            head: 0,
            split: 0,
            allstolen: false,
            #[cfg(feature = "metrics")]
            metrics,
        }
    }
}
impl<T> DequeShared<T> {
    fn new(buffer_size: usize, #[cfg(feature = "metrics")] metrics: Arc<Metrics>) -> Self {
        Self {
            split_tail: AtomicUsize::new(0),
            splitreq: AtomicBool::new(false),
            allstolen: AtomicBool::new(false),
            buffer: DequeBuffer::new(buffer_size),
            #[cfg(feature = "metrics")]
            metrics,
        }
    }
}

pub struct Worker<T> {
    owned: CachePadded<DequeOwned>,
    shared: CachePadded<Arc<CachePadded<DequeShared<T>>>>,
}
impl<T> Worker<T> {
    #[inline(always)]
    fn set_shared_split(&mut self, _old_s: usize, new_s: usize) {
        #[cfg(not(feature = "unsafe_atomic"))]
        {
            self.shared
                .split_tail
                .fetch_xor((_old_s ^ new_s) << 32, Ordering::Release);
        }
        #[cfg(feature = "unsafe_atomic")]
        {
            use std::sync::atomic::AtomicU32;
            // NOTE: we assume x86 endianness here
            let ptr = (self.shared.split_tail.as_ptr() as *mut u32).wrapping_add(1);
            unsafe { AtomicU32::from_ptr(ptr).store(new_s as u32, Ordering::Relaxed) };
        }
    }
    #[cold]
    fn grow_shared(&mut self) {
        owner_invariants!(self, "grow_shared");
        #[cfg(feature = "metrics")]
        self.owned.metrics.grows.fetch_add(1, Ordering::Relaxed);
        let s = self.owned.split;
        let new_s = (s + self.owned.head + 1) / 2;
        // split = new_s
        self.set_shared_split(s, new_s);
        self.owned.split = new_s;
        #[cfg(feature="stronger_splitreq")]
        self.shared.splitreq.store(false, Ordering::SeqCst);
        #[cfg(not(feature="stronger_splitreq"))]
        self.shared.splitreq.store(false, Ordering::Relaxed);
    }
    #[cold]
    #[inline(never)]
    fn shrink_shared(&mut self) -> bool {
        owner_invariants!(self, "shrink_shared");
        // (t, s) = (tail, split)
        let st = self.shared.split_tail.load(Ordering::Relaxed);
        let t = st as u32 as usize;
        let s = (st >> 32) as u32 as usize;
        if t != s {
            #[cfg(feature = "metrics")]
            self.owned
                .metrics
                .shrink_not_allstolen1
                .fetch_add(1, Ordering::Relaxed);
            let new_s = (t + s) / 2;
            // split = new_s
            let t = self
                .shared
                .split_tail
                .fetch_xor((s ^ new_s) << 32, Ordering::SeqCst) as u32 as usize;
            // MFENCE comes from Ordering::SeqCst
            if t != s {
                #[cfg(feature = "metrics")]
                self.owned
                    .metrics
                    .shrink_not_allstolen2
                    .fetch_add(1, Ordering::Relaxed);
                if t > new_s {
                    // tasks were stolen while moving the splitpoint, so move it again.
                    let newer_s = (t + s) / 2;
                    // split = newer_s
                    self.set_shared_split(new_s, newer_s);
                    self.owned.split = newer_s;
                    #[cfg(feature = "metrics")]
                    self.owned
                        .metrics
                        .shrink_fix_tail
                        .fetch_add(1, Ordering::Relaxed);
                } else {
                    self.owned.split = new_s;
                }
                return false;
            }
        }
        #[cfg(feature = "metrics")]
        self.owned
            .metrics
            .shrink_allstolen
            .fetch_add(1, Ordering::Relaxed);
        // tail == head
        self.shared.allstolen.store(true, Ordering::Relaxed);
        self.owned.allstolen = true;
        true
    }

    #[cold]
    fn task_stack_overflow() -> ! {
        panic!("Task Stack Overflow")
    }
    #[cold]
    fn push_allstolen(&mut self) {
        // (tail, split) = (head-1, head)
        let h = self.owned.head as u32;
        self.shared
            .split_tail
            .store((h as usize) << 32 | (h - 1) as usize, Ordering::Release);
        self.shared.allstolen.store(false, Ordering::Relaxed);
        #[cfg(feature="stronger_splitreq")]
        if self.shared.splitreq.load(Ordering::SeqCst) {
            self.shared.splitreq.store(false, Ordering::SeqCst);
        }
        #[cfg(not(feature="stronger_splitreq"))]
        if self.shared.splitreq.load(Ordering::Relaxed) {
            self.shared.splitreq.store(false, Ordering::Relaxed);
        }
        self.owned.split = self.owned.head;
        self.owned.allstolen = false;
    }
    pub fn push(&mut self, task: T) {
        owner_invariants!(self, "push");
        if self.owned.head >= self.shared.buffer.size {
            Self::task_stack_overflow();
        }
        let h = self.owned.head;
        qassert!(
            self.thief_flag(h) == THIEF_NONE,
            self,
            "thief flag @ {} not reset properly, it's {}",
            h,
            (self.thief_flag(h))
        );
        self.shared.buffer.put(h, task);
        self.owned.head += 1;
        if self.owned.allstolen {
            self.push_allstolen();
        } else {
            #[cfg(feature="stronger_splitreq")]
            if self.shared.splitreq.load(Ordering::SeqCst) {
                self.grow_shared();
            }
            #[cfg(not(feature="stronger_splitreq"))]
            if self.shared.splitreq.load(Ordering::Relaxed) {
                self.grow_shared();
            }
        }
    }
    pub fn pop(&mut self) -> Pop<T> {
        owner_invariants!(self, "pop");
        if self.owned.allstolen {
            // there's no work left
            return Pop::Stolen(self.owned.head - 1);
        }
        if self.owned.split == self.owned.head {
            if self.shrink_shared() {
                // all tasks are stolen
                return Pop::Stolen(self.owned.head - 1);
            }
        }
        qassert!(self.owned.head > 0, self, "pop() on an empty deque");
        self.owned.head -= 1;
        #[cfg(feature="stronger_splitreq")]
        if self.shared.splitreq.load(Ordering::SeqCst) {
            self.grow_shared();
        }
        #[cfg(not(feature="stronger_splitreq"))]
        if self.shared.splitreq.load(Ordering::Relaxed) {
            self.grow_shared();
        }
        Pop::Work(self.shared.buffer.take(self.owned.head))
    }

    pub fn thief_flag(&self, i: usize) -> usize {
        self.shared.buffer.thief[i].load(Ordering::Acquire)
    }
    pub fn pop_stolen(&mut self) -> T {
        qassert!(self.owned.head > 0, self, "pop_stolen on empty queue");
        self.owned.head -= 1;
        if !self.owned.allstolen {
            self.shared.allstolen.store(true, Ordering::Relaxed);
            self.owned.allstolen = true;
        }
        let h = self.owned.head;
        qassert!(
            self.thief_flag(h) == THIEF_FINISHED,
            self,
            "pop_stolen of unfinished task"
        );
        self.shared.buffer.thief[h].store(THIEF_NONE, Ordering::Relaxed);
        self.shared.buffer.take(h)
    }

    #[inline(always)]
    pub fn sync_fast(&mut self) -> Option<T> {
        owner_invariants!(self, "sync_fast");
        #[cfg(feature="stronger_splitreq")]
        if !self.shared.splitreq.load(Ordering::SeqCst) && self.owned.split < self.owned.head {
            self.owned.head -= 1;
            Some(self.shared.buffer.take(self.owned.head))
        } else {
            None
        }
        #[cfg(not(feature="stronger_splitreq"))]
        if !self.shared.splitreq.load(Ordering::Relaxed) && self.owned.split < self.owned.head {
            self.owned.head -= 1;
            Some(self.shared.buffer.take(self.owned.head))
        } else {
            None
        }
    }
}

pub struct Stealer<T> {
    shared: Arc<CachePadded<DequeShared<T>>>,
}
#[derive(Debug, PartialEq, Eq)]
pub enum Steal<T> {
    Empty,
    Success(T),
    Retry,
}
impl<T> Stealer<T> {
    #[inline(always)]
    pub fn steal_common(&self, thief_id: usize) -> Steal<usize> {
        if self.shared.allstolen.load(Ordering::Relaxed) {
            return Steal::Empty;
        }
        // (t, s) = (tail, split)
        let st = self.shared.split_tail.load(Ordering::Relaxed);
        let t = st as u32 as usize;
        let s = (st >> 32) as u32 as usize;
        if t < s {
            // cas((tail, split), (t, s), (t+1, s)),
            if self
                .shared
                .split_tail
                .compare_exchange_weak(st, st + 1, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
            {
                muteable_assert!(
                    self.shared.buffer.thief[t].load(Ordering::Relaxed) == THIEF_NONE,
                    {
                        eprintln!("Thief flag not NONE while stealing");
                    }
                );
                self.shared.buffer.thief[t].store(thief_id, Ordering::Relaxed);
                return Steal::Success(t);
            } else {
                return Steal::Retry;
            }
        }
        #[cfg(feature="stronger_splitreq")]
        if !self.shared.splitreq.load(Ordering::SeqCst) {
            #[cfg(feature = "metrics")]
            self.shared
                .metrics
                .splitreqs
                .fetch_add(1, Ordering::Relaxed);
            self.shared.splitreq.store(true, Ordering::SeqCst);
        }
        #[cfg(not(feature="stronger_splitreq"))]
        if !self.shared.splitreq.load(Ordering::Relaxed) {
            #[cfg(feature = "metrics")]
            self.shared
                .metrics
                .splitreqs
                .fetch_add(1, Ordering::Relaxed);
            self.shared.splitreq.store(true, Ordering::Relaxed);
        }
        Steal::Empty
    }
    #[cfg(not(feature = "unsafe_steal"))]
    pub fn steal(&self, thief_id: usize) -> Steal<(usize, T)> {
        match self.steal_common(thief_id) {
            Steal::Success(i) => Steal::Success((i, self.shared.buffer.take(i))),
            Steal::Retry => Steal::Retry,
            Steal::Empty => Steal::Empty,
        }
    }
    #[cfg(feature = "unsafe_steal")]
    pub fn steal(&self, thief_id: usize) -> Steal<(usize, *mut T)> {
        match self.steal_common(thief_id) {
            Steal::Success(i) => Steal::Success((i, self.shared.buffer.ptr(i))),
            Steal::Retry => Steal::Retry,
            Steal::Empty => Steal::Empty,
        }
    }

    #[inline(always)]
    pub fn steal_finished(&self, i: usize, #[cfg(not(feature = "unsafe_steal"))] task: T) {
        #[cfg(not(feature = "unsafe_steal"))]
        self.shared.buffer.put(i, task);
        self.shared.buffer.thief[i].store(THIEF_FINISHED, Ordering::Release);
    }
}
impl<T> Clone for Stealer<T> {
    fn clone(&self) -> Self {
        Self {
            shared: Arc::clone(&self.shared),
        }
    }
}

pub fn deque<T>(
    buffer_size: usize,
    #[cfg(feature = "metrics")] metrics: Arc<Metrics>,
) -> (Worker<T>, Stealer<T>) {
    let owned = CachePadded::from(DequeOwned::new(
        #[cfg(feature = "metrics")]
        metrics.clone(),
    ));
    let shared = Arc::new(CachePadded::from(DequeShared::new(
        buffer_size,
        #[cfg(feature = "metrics")]
        metrics,
    )));
    let shared_ref = Arc::clone(&shared);
    (
        Worker {
            owned,
            shared: CachePadded::from(shared),
        },
        Stealer { shared: shared_ref },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prng::Lfsr;

    #[test]
    fn push_pop() {
        let (mut w, _) = deque::<usize>(
            10,
            #[cfg(feature = "metrics")]
            Arc::new(Metrics::default()),
        );
        w.push(123);
        w.push(456);
        assert_eq!(w.pop(), Pop::Work(456));
        assert_eq!(w.pop(), Pop::Work(123));
    }
    #[test]
    fn stealing() {
        let (mut w, s) = deque::<usize>(
            10,
            #[cfg(feature = "metrics")]
            Arc::new(Metrics::default()),
        );
        w.push(123);
        w.push(456);
        // first the shared section is empty
        assert_eq!(s.steal(0), Steal::Empty);
        // this call to pop() should grow it
        assert_eq!(w.pop(), Pop::Work(456));
        // now the steal should work
        let Steal::Success((t, task)) = s.steal(42) else {
            panic!("steal() failed unexpectedly");
        };
        assert_eq!(t, 0);
        #[cfg(not(feature = "unsafe_steal"))]
        assert_eq!(task, 123);
        #[cfg(feature = "unsafe_steal")]
        assert_eq!(unsafe { *task }, 123);
        // the thief ID should be written
        assert_eq!(w.thief_flag(t), 42);
        assert_eq!(w.pop(), Pop::Stolen(t));

        #[cfg(not(feature = "unsafe_steal"))]
        s.steal_finished(0, 789);
        // steal_finished should update the flag
        assert_eq!(w.thief_flag(0), THIEF_FINISHED);
        #[cfg(not(feature = "unsafe_steal"))]
        // the steal_finished task should be in the buffer
        assert_eq!(w.pop_stolen(), 789)
    }

    fn stressor(prng: &mut Lfsr) {
        let _ = crossbeam_utils::thread::scope(|scope| {
            const DEQUE_SIZE: usize = 64;
            const N_THIEVES: usize = 32;
            let (mut w, sh) = deque::<usize>(
                DEQUE_SIZE,
                #[cfg(feature = "metrics")]
                Arc::new(Metrics::default()),
            );
            let keep_going = Arc::new(AtomicUsize::new(1));
            let continue_flag = Arc::clone(&keep_going);
            let bar = Arc::new(std::sync::Barrier::new(N_THIEVES + 1));
            // thieves
            for id in 0..N_THIEVES {
                let continue_flag = Arc::clone(&keep_going);
                let s = sh.clone();
                let b = bar.clone();
                scope.spawn(move |_| {
                    b.wait();
                    while continue_flag.load(Ordering::Relaxed) != 0 {
                        if let Steal::Success((i, _t)) = s.steal(id) {
                            #[cfg(not(feature = "unsafe_steal"))]
                            s.steal_finished(i, _t);
                            #[cfg(feature = "unsafe_steal")]
                            s.steal_finished(i);
                        }
                    }
                });
            }
            // worker
            scope.spawn(move |_| {
                bar.wait();
                // fill and drain the queue a bunch of times
                let mut space = DEQUE_SIZE;
                while space > 0 {
                    for _ in 0..space {
                        w.push(w.owned.head);
                    }
                    let reclaim = prng.next(space);
                    for _ in 0..reclaim {
                        let expected = w.owned.head - 1;
                        if let Some(x) = w.sync_fast() {
                            assert_eq!(x, expected);
                        } else {
                            match w.pop() {
                                Pop::Work(x) => {
                                    assert_eq!(x, expected);
                                }
                                Pop::Stolen(i) => {
                                    loop {
                                        let thief = w.thief_flag(i);
                                        if thief == THIEF_FINISHED {
                                            break;
                                        } else {
                                            assert!(thief == THIEF_NONE || thief < N_THIEVES);
                                        }
                                    }
                                    assert_eq!(i, w.owned.head - 1);
                                    let x = w.pop_stolen();
                                    assert_eq!(x, expected);
                                    assert_eq!(i, w.owned.head);
                                    assert_eq!(w.thief_flag(i), THIEF_NONE);
                                }
                            }
                        }
                    }
                    space = reclaim;
                }
                continue_flag.store(0, Ordering::Relaxed);
            });
        });
    }
    #[test]
    fn stress() {
        let mut prng = Lfsr::new(0);
        for _ in 0..10000 {
            // crate::log!("stressor()");
            stressor(&mut prng);
        }
    }
}
