use std::marker::PhantomData;
use std::ops::Drop;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

extern crate lace_macros;
pub use lace_macros::lace_task;

#[cfg(any(feature = "thread_spread", feature = "thread_nospread"))]
extern crate hwloc2;
#[cfg(any(feature = "thread_spread", feature = "thread_nospread"))]
use hwloc2::{CpuBindFlags, ObjectType, Topology};

#[cfg(feature = "numa_awareness")]
extern crate hwloc2;
#[cfg(feature = "numa_awareness")]
use hwloc2::{CpuBindFlags, ObjectType, Topology};

#[cfg(feature = "metrics")]
mod metrics;
#[cfg(feature = "metrics")]
use metrics::Metrics;

mod arena;
use arena::picked::Arena;
mod common;
use common::{Function, THIEF_FINISHED, THIEF_NONE};
mod prng;
mod task;
use task::picked::{Task, TypeErased as TypeErasedTask};

#[cfg(not(feature = "chase_lev"))]
mod lace_deque;
#[cfg(not(feature = "chase_lev"))]
use lace_deque::{Pop, Steal, Stealer, Worker as Deque};
#[cfg(not(feature = "chase_lev"))]
mod buffer;

#[cfg(feature = "chase_lev")]
extern crate crossbeam_deque;
#[cfg(feature = "chase_lev")]
use arena::picked::ArenaBox;
#[cfg(feature = "chase_lev")]
use crossbeam_deque::{Steal, Stealer, Worker as Deque};
#[cfg(feature = "chase_lev")]
use std::sync::atomic::AtomicUsize;

#[allow(unused_macros)]
macro_rules! wlog {
    ($self:expr, $msg:expr) => {
        log!("[WORKER {}]: {}", (($self).id), $msg);
    };
    ($self:expr, $fmt:expr, $($args:expr),*) => {
        wlog!($self, format!($fmt, $($args),*));
    }
}

#[cfg(not(feature = "chase_lev"))]
type TaskDeque = Deque<TypeErasedTask>;
#[cfg(not(feature = "chase_lev"))]
type TaskStealer = Stealer<TypeErasedTask>;
#[cfg(feature = "chase_lev")]
type TaskDeque = Deque<(*mut TypeErasedTask, Arc<AtomicUsize>)>;
#[cfg(feature = "chase_lev")]
type TaskStealer = Stealer<(*mut TypeErasedTask, Arc<AtomicUsize>)>;

pub struct Worker {
    id: usize,
    keep_going: Arc<AtomicBool>,
    arena: Arena,
    prng: prng::Lfsr,
    queue: TaskDeque,
    steal_handles: Vec<TaskStealer>,
    #[cfg(feature = "chase_lev")]
    recycle_thief: Vec<(Arc<AtomicUsize>, Arc<AtomicUsize>)>,
    #[cfg(feature = "metrics")]
    metrics: Arc<Metrics>,
}
impl Worker {
    pub fn new(
        id: usize,
        keep_going: Arc<AtomicBool>,
        #[cfg(feature = "metrics")] metrics: Arc<Metrics>,
    ) -> (Self, TaskStealer) {
        #[cfg(not(feature = "chase_lev"))]
        let (queue, stealer) = lace_deque::deque(
            common::BUFFER_SIZE,
            #[cfg(feature = "metrics")]
            metrics.clone(),
        );
        #[cfg(feature = "chase_lev")]
        let queue = Deque::new_lifo();
        #[cfg(feature = "chase_lev")]
        let stealer = queue.stealer();

        (
            Self {
                id,
                arena: Arena::new(),
                queue,
                prng: prng::Lfsr::new(id),
                steal_handles: Vec::new(),
                keep_going,
                #[cfg(feature = "chase_lev")]
                recycle_thief: Vec::new(),
                #[cfg(feature = "metrics")]
                metrics: metrics.clone(),
            },
            stealer,
        )
    }

    #[cfg(test)]
    pub fn mock() -> Self {
        Self::new(
            0,
            Arc::new(AtomicBool::from(true)),
            #[cfg(feature = "metrics")]
            Arc::new(Metrics::default()),
        )
        .0
    }
}

#[must_use]
pub struct SpawnToken<'task, I, O> {
    #[cfg(feature = "chase_lev")]
    task: *mut TypeErasedTask,
    #[cfg(feature = "chase_lev")]
    thief: Arc<AtomicUsize>,
    _lt: PhantomData<&'task ()>,
    _i: PhantomData<I>,
    _o: PhantomData<O>,
}

#[cfg(not(feature = "chase_lev"))]
impl Worker {
    #[cold]
    #[inline(never)]
    fn sync_slow<I, O>(&mut self) -> O {
        match self.queue.pop() {
            Pop::Work(task) => {
                let task: Task<I, O> = task.unerase();
                task.execute(self)
            }
            Pop::Stolen(i) => {
                let mut thief = THIEF_NONE;
                // busy-wait for the thief to write their ID
                while thief == THIEF_NONE {
                    thief = self.queue.thief_flag(i);
                }
                #[cfg(feature = "steal_backoff")]
                let mut backoff: usize = 0;
                #[cfg(feature = "leapfrog_random")]
                let mut attempts: usize = 32; // same as in the C implementation
                while thief != THIEF_FINISHED {
                    #[cfg(not(feature = "leapfrog_random"))]
                    self.leapfrog(thief);
                    #[cfg(feature = "leapfrog_random")]
                    if !self.leapfrog(thief) {
                        attempts -= 1;
                        if attempts == 0 {
                            attempts = 32;
                            self.steal_random();
                        }
                    }
                    #[cfg(feature = "steal_backoff")]
                    {
                        for _ in 0..backoff {
                            std::hint::spin_loop();
                        }
                        backoff = ((backoff + 1) * 2).min(1024);
                    }
                    thief = self.queue.thief_flag(i)
                }
                let task = self.queue.pop_stolen();
                let mut task: Task<I, O> = task.unerase();
                task.take_output(&mut self.arena)
            }
        }
    }
    #[inline(always)]
    pub fn sync<I, O>(&mut self, _: SpawnToken<'_, I, O>) -> O {
        // shortcut for the common case
        if let Some(task) = self.queue.sync_fast() {
            let task: Task<I, O> = task.unerase();
            task.execute(self)
        } else {
            self.sync_slow::<I, O>()
        }
    }
    #[inline(always)]
    pub fn spawn<'task, I: 'task, O: 'task>(
        &mut self,
        task: Function<I, O>,
        input: I,
    ) -> SpawnToken<'task, I, O> {
        #[cfg(feature = "metrics")]
        self.metrics.tasks.fetch_add(1, Ordering::Relaxed);
        self.queue
            .push(Task::new(&mut self.arena, task, input).erase());
        SpawnToken {
            _lt: PhantomData,
            _i: PhantomData,
            _o: PhantomData,
        }
    }

    #[inline(always)]
    pub fn join<A, B, RA, RB>(
        &mut self,
        task_a: Function<A, RA>,
        a: A,
        task_b: Function<B, RB>,
        b: B,
    ) -> (RA, RB) {
        let _ = self.spawn(task_a, a);
        // execute task B synchronously,
        // hopefully task A will get stolen in the meantime
        let result_b: RB = task_b(self, b);
        // synchronize on task A
        let result_a: RA = if let Some(task) = self.queue.sync_fast() {
            let mut task: Task<A, RA> = task.unerase();
            let ipt = task.take_input(&mut self.arena);
            task_a(self, ipt)
        } else {
            self.sync_slow::<A, RA>()
        };
        (result_a, result_b)
    }

    fn steal_work(&mut self, victim: usize) -> Steal<()> {
        match self.steal_handles[victim].steal(self.id) {
            #[cfg(not(feature = "unsafe_steal"))]
            Steal::Success((i, mut task)) => {
                #[cfg(feature = "metrics")]
                self.metrics.steal_success.fetch_add(1, Ordering::Relaxed);
                task.execute_stolen(self);
                self.steal_handles[victim].steal_finished(i, task);
                Steal::Success(())
            }
            #[cfg(feature = "unsafe_steal")]
            Steal::Success((i, task)) => {
                #[cfg(feature = "metrics")]
                self.metrics.steal_success.fetch_add(1, Ordering::Relaxed);
                unsafe { (*task).execute_stolen(self) };
                self.steal_handles[victim].steal_finished(i);
                Steal::Success(())
            }
            Steal::Retry => {
                #[cfg(feature = "metrics")]
                self.metrics.steal_busy.fetch_add(1, Ordering::Relaxed);
                Steal::Retry
            }
            Steal::Empty => {
                #[cfg(feature = "metrics")]
                self.metrics.steal_empty.fetch_add(1, Ordering::Relaxed);
                Steal::Empty
            }
        }
    }
}

#[cfg(feature = "chase_lev")]
unsafe impl Send for Worker {}
#[cfg(feature = "chase_lev")]
impl Worker {
    #[cold]
    fn new_thief_slot() -> (Arc<AtomicUsize>, Arc<AtomicUsize>) {
        let a = Arc::new(AtomicUsize::new(THIEF_NONE));
        (a.clone(), a)
    }
    #[inline(always)]
    pub fn spawn<'task, I: 'task, O: 'task>(
        &mut self,
        task: Function<I, O>,
        input: I,
    ) -> SpawnToken<'task, I, O> {
        let task = Task::new(&mut self.arena, task, input).erase();
        let task_box = self.arena.alloc(task).into_mut_ptr();
        let (thief_a, thief_b) = if let Some(x) = self.recycle_thief.pop() {
            x
        } else {
            Self::new_thief_slot()
        };
        #[cfg(feature = "metrics")]
        self.metrics.tasks.fetch_add(1, Ordering::Relaxed);
        self.queue.push((task_box, thief_a));
        SpawnToken {
            task: task_box,
            thief: thief_b,
            _lt: PhantomData,
            _i: PhantomData,
            _o: PhantomData,
        }
    }
    #[inline(always)]
    pub fn sync<I, O>(&mut self, tkn: SpawnToken<'_, I, O>) -> O {
        match self.queue.pop() {
            Some((_ptr, thief_b)) => {
                muteable_assert!(_ptr == tkn.task, {
                    eprintln!(
                        "Task Pointer Is Not What It Should Be (spawn/sync FIFO order violation)"
                    );
                });
                self.recycle_thief.push((tkn.thief, thief_b));
                // the task was never stolen, just deallocate it
                let task = self.arena.take(ArenaBox::from_mut_ptr(tkn.task));
                let task: Task<I, O> = task.unerase();
                task.execute(self)
            }
            None => {
                // it was stolen, wait for the thief to write their ID
                let mut thief = THIEF_NONE;
                while thief == THIEF_NONE {
                    thief = tkn.thief.load(Ordering::Relaxed)
                }
                #[cfg(feature = "steal_backoff")]
                let mut backoff: usize = 0;
                #[cfg(feature = "leapfrog_random")]
                let mut attempts: usize = 32; // same as in the C implementation
                while thief != THIEF_FINISHED {
                    #[cfg(not(feature = "leapfrog_random"))]
                    self.leapfrog(thief);
                    #[cfg(feature = "leapfrog_random")]
                    if !self.leapfrog(thief) {
                        attempts -= 1;
                        if attempts == 0 {
                            attempts = 32;
                            self.steal_random();
                        }
                    }
                    #[cfg(feature = "steal_backoff")]
                    {
                        for _ in 0..backoff {
                            std::hint::spin_loop();
                        }
                        backoff = ((backoff + 1) * 2).min(1024);
                    }
                    thief = tkn.thief.load(Ordering::Acquire);
                }
                tkn.thief.store(THIEF_NONE, Ordering::Relaxed);
                let task = self.arena.take(ArenaBox::from_mut_ptr(tkn.task));
                let mut task: Task<I, O> = task.unerase();
                task.take_output(&mut self.arena)
            }
        }
    }

    #[inline(always)]
    pub fn join<A, B, RA, RB>(
        &mut self,
        task_a: Function<A, RA>,
        a: A,
        task_b: Function<B, RB>,
        b: B,
    ) -> (RA, RB) {
        let tkn = self.spawn(task_a, a);
        // execute task B synchronously,
        // hopefully task A will get stolen in the meantime
        let result_b: RB = task_b(self, b);
        // synchronize on task A
        let result_a: RA = self.sync::<A, RA>(tkn);
        (result_a, result_b)
    }

    fn steal_work(&mut self, victim: usize) -> Steal<()> {
        match self.steal_handles[victim].steal() {
            Steal::Success((task, thief_slot)) => {
                #[cfg(feature = "metrics")]
                self.metrics.steal_success.fetch_add(1, Ordering::Relaxed);
                thief_slot.store(self.id, Ordering::Relaxed);
                unsafe { (*task).execute_stolen(self) };
                thief_slot.store(THIEF_FINISHED, Ordering::Release);
                Steal::Success(())
            }
            Steal::Retry => {
                #[cfg(feature = "metrics")]
                self.metrics.steal_busy.fetch_add(1, Ordering::Relaxed);
                Steal::Retry
            }
            Steal::Empty => {
                #[cfg(feature = "metrics")]
                self.metrics.steal_empty.fetch_add(1, Ordering::Relaxed);
                Steal::Empty
            }
        }
    }
}

impl Worker {
    #[cfg(any(feature = "numa_awareness", feature = "thread_spread", feature = "thread_nospread"))]
    fn numa_bind(&mut self, pu: usize) {
        let mut topo = Topology::new().unwrap();
        let pu_set = topo.objects_with_type(&ObjectType::PU).unwrap()[pu]
            .cpuset()
            .unwrap();
        topo.set_cpubind(pu_set, CpuBindFlags::CPUBIND_THREAD)
            .expect("Failed to bind thread to CPU");
    }

    #[inline(always)]
    fn leapfrog(&mut self, thief: usize) -> bool {
        match self.steal_work(thief) {
            Steal::Success(_) => {
                #[cfg(feature = "metrics")]
                self.metrics.leap_success.fetch_add(1, Ordering::Relaxed);
                true
            }
            Steal::Empty => {
                #[cfg(feature = "metrics")]
                self.metrics.leap_empty.fetch_add(1, Ordering::Relaxed);
                false
            }
            Steal::Retry => {
                #[cfg(feature = "metrics")]
                self.metrics.leap_busy.fetch_add(1, Ordering::Relaxed);
                true
            }
        }
    }

    #[inline(always)]
    fn steal_random(&mut self) {
        let n = self.steal_handles.len();
        // steal from a random victim
        let mut victim = self.id;
        while victim == self.id {
            victim = self.prng.next(n);
        }
        let _ = self.steal_work(victim);
    }

    fn thread(mut self, #[cfg(any(feature = "numa_awareness", feature = "thread_spread", feature = "thread_nospread"))] pu: usize) -> thread::JoinHandle<()> {
        thread::Builder::new()
            .name("Lace Worker".to_string())
            .stack_size(1024 * 1024 * 16)
            .spawn(move || {
                #[cfg(any(feature = "numa_awareness", feature = "thread_spread", feature = "thread_nospread"))]
                self.numa_bind(pu);
                // these don't change over the lifetime of the thread,
                // so we can safely make local copies
                while self.keep_going.load(Ordering::Relaxed) {
                    self.steal_random();
                }
            })
            .expect("Failed to spawn Lace Worker Thread")
    }
}

pub struct Lace {
    root_worker: Worker,
    handles: Vec<thread::JoinHandle<()>>,
    keep_going: Arc<AtomicBool>,
    #[cfg(feature = "metrics")]
    worker_metrics: Vec<Arc<Metrics>>,
}
impl Lace {
    pub fn init(n: usize) -> Self {
        assert!(n != 0, "invalid number of workers");
        let keep_going = Arc::from(AtomicBool::new(true));
        let mut workers = Vec::new();
        let mut steal_handles = Vec::new();
        #[cfg(feature = "metrics")]
        let mut worker_metrics = Vec::new();

        for id in 0..n {
            #[cfg(feature = "metrics")]
            let metrics = Arc::new(Metrics::default());
            let (w, s) = Worker::new(
                id,
                keep_going.clone(),
                #[cfg(feature = "metrics")]
                metrics.clone(),
            );
            steal_handles.push(s);
            workers.push(w);
            #[cfg(feature = "metrics")]
            worker_metrics.push(metrics);
        }
        // give each worker a steal handle for the others
        for w in workers.iter_mut() {
            w.steal_handles = steal_handles.clone();
        }
        // only start n-1 extra threads, the main thread is the root worker
        let mut out = Self {
            root_worker: workers.remove(0),
            handles: Vec::new(),
            keep_going,
            #[cfg(feature = "metrics")]
            worker_metrics,
        };
        #[cfg(any(feature = "thread_spread", feature = "thread_nospread"))]
        {
            let topo = Topology::new().unwrap();
            if !topo.support().cpu().set_current_thread() {
                panic!("NUMA awareness enabled but thread binding not supported");
            }
            let mut allocated_pu = Vec::with_capacity(n);
            if let Ok(sockets) = topo.objects_with_type(&ObjectType::Package) {
                let mut socket_pus = Vec::new();
                for (si, socket) in sockets.iter().enumerate() {
                    socket_pus.push(Vec::new());
                    let mut ci = 0;
                    for child in socket.children().iter() {
                        for gchild in child.children().iter() {
                            for ggchild in gchild.children().iter() {
                                for core in ggchild.children().iter() {
                                    assert_eq!( core.object_type(), ObjectType::Core );
                                    socket_pus[si].push(Vec::new());
                                    for pu in core.children().iter() {
                                        assert_eq!( pu.object_type(), ObjectType::PU );
                                        socket_pus[si][ci].push(pu.logical_index());
                                    }
                                    ci += 1;
                                }
                            }
                        }
                    }
                }
                #[cfg(feature = "thread_spread")]
                {
                    let mut ci = Vec::with_capacity(socket_pus.len());
                    for _ in 0..socket_pus.len() {
                        ci.push(0);
                    }
                    let mut si = 0;
                    while allocated_pu.len() < n {
                        allocated_pu.push(socket_pus[si][ci[si]][0] as usize);
                        ci[si] = (ci[si] + 1) % socket_pus[si].len();
                        si = (si + 1) % socket_pus.len();
                    }
                }
                #[cfg(feature = "thread_nospread")]
                for i in 0..n {
                    allocated_pu.push(socket_pus[0][(i/2) % socket_pus[0].len()][i%2] as usize);
                }
            }
            println!("allocated_pu: {allocated_pu:?}");
            out.root_worker.numa_bind(allocated_pu[0]);
            for (i, w) in workers.drain(..).enumerate() {
                out.handles.push(w.thread(allocated_pu[i + 1]));
            }
        }
        #[cfg(feature = "numa_awareness")]
        {
            let topo = Topology::new().unwrap();
            if !topo.support().cpu().set_current_thread() {
                panic!("NUMA awareness enabled but thread binding not supported");
            }
            // distribute the threads over the processing units
            let mut allocated_pu = Vec::with_capacity(n);
            if let Ok(cores) = topo.objects_with_type(&ObjectType::Core) {
                // minimize the number of threads that share a CPU
                let mut core_pus = Vec::new();
                for (ci, core) in cores.iter().enumerate() {
                    if let Some(c) = core.first_child() {
                        core_pus.push((ci, c));
                    }
                }
                // don't use core 0 for one worker, there
                // are usually other things running there and that would skew speedup factor
                // computation unrealistically in favor of the NUMA aware version
                core_pus.reverse();
                // For example, with 2 cores with 2 PUs each.
                // [0], [0, 2], [0, 2, 1], [0, 2, 1, 3], [0, 2, 1, 3, 0], etc.
                while allocated_pu.len() < n {
                    for (ci, c) in core_pus.iter_mut() {
                        assert_eq!(c.object_type(), ObjectType::PU);
                        let pu = c.logical_index() as usize;
                        allocated_pu.push(pu);
                        if allocated_pu.len() == n {
                            break;
                        }
                        // move on to the next PU on the core
                        if let Some(sib) = c.next_sibling() {
                            *c = sib;
                        } else {
                            *c = cores[*ci].first_child().unwrap();
                        }
                    }
                }
            } else if let Ok(pus) = topo.objects_with_type(&ObjectType::PU) {
                // distribute evenly
                for id in 0..n {
                    allocated_pu.push(pus[id % pus.len()].logical_index() as usize);
                }
            } else {
                panic!("NUMA awareness enabled but could not determine CPU or PU counts")
            }
            out.root_worker.numa_bind(allocated_pu[0]);
            for (i, w) in workers.drain(..).enumerate() {
                out.handles.push(w.thread(allocated_pu[i + 1]));
            }
        }
        #[cfg(not(any(feature = "numa_awareness", feature = "thread_spread", feature = "thread_nospread")))]
        for w in workers.drain(..) {
            out.handles.push(w.thread());
        }
        out
    }

    #[inline(always)]
    pub fn run<O>(&mut self, f: impl FnOnce(&mut Worker) -> O) -> O {
        f(&mut self.root_worker)
    }

    pub fn stop(&mut self) {
        self.keep_going.store(false, Ordering::Relaxed);
        for handle in self.handles.drain(..) {
            handle.join().expect("failed to stop worker thread");
        }
    }

    #[cfg(feature = "metrics")]
    #[allow(dead_code)] // it's used by the benchmarks
    pub fn summarize(&self, normf: usize) {
        let mut total = Metrics::default();
        for (id, m) in self.worker_metrics.iter().enumerate() {
            m.add_into(&mut total);
            let m = m.normalized(normf);
            println!("worker {id:2}: {:?}", m);
        }
        println!("total: {:?}", total.normalized(normf));
    }
}
// safeguard in case people forget to call stop()
impl Drop for Lace {
    fn drop(&mut self) {
        self.stop()
    }
}

#[macro_export]
macro_rules! lace_run {
    ($inst:expr, $($task:ident)::+($($args:expr),*)) => {
        ($inst).run(|__lace_task_worker| $($task)::+(__lace_task_worker, ($($args),*)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[lace_task]
    fn smoke_mixed_args(_: f32, _: bool) -> usize {
        321
    }
    #[lace_task]
    fn smoke_joining_task(x: &mut usize) -> usize {
        if *x < 15 {
            *x += 1;
            let (a, b) = join!(smoke_mixed_args(0.4, false), smoke_joining_task(x));
            a.min(b)
        } else {
            123
        }
    }
    #[test]
    fn smoke() {
        let mut lace = Lace::init(4);
        let mut x = 11;
        let ret = lace_run!(lace, smoke_joining_task(&mut x));
        assert_eq!(ret, 123);
        assert_eq!(x, 15);
    }

    #[lace_task]
    fn dfs(n: usize) -> usize {
        if n == 0 {
            return 1;
        }
        let mut tokens = Vec::new();
        for _ in 0..n {
            tokens.push(spawn!(dfs(n - 1)));
        }
        let mut total = 0;
        while let Some(tkn) = tokens.pop() {
            total += sync!(tkn);
        }
        total
    }
    #[test]
    fn largerun() {
        let mut lace = Lace::init(4);
        let x = lace_run!(lace, dfs(10));
        assert_eq!(x, 10 * 9 * 8 * 7 * 6 * 5 * 4 * 3 * 2 * 1);
    }
    #[test]
    fn manysmallruns() {
        let mut lace = Lace::init(4);
        const N: usize = 1000;
        for _ in 0..N {
            let x = lace_run!(lace, dfs(8));
            assert_eq!(x, 8 * 7 * 6 * 5 * 4 * 3 * 2 * 1);
        }
    }
}
