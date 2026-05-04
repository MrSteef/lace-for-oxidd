use crate::arena::picked::{Arena, ArenaBox};
use crate::common::{Function, TASK_INLINE_SIZE};
use crate::Worker;
use std::mem::{size_of, transmute, ManuallyDrop, MaybeUninit};

pub struct Task<I, O> {
    f: Function<I, O>,
    io: TaskIO<I, O>,
    execute_stolen: fn(&mut TypeErased, &mut Worker),
}
union TaskIO<I, O> {
    inline: [u8; TASK_INLINE_SIZE],
    boxed: ManuallyDrop<ArenaBox<I>>,
    output: ManuallyDrop<(Option<ArenaBox<I>>, Box<O>)>,
}
impl<I, O> Task<I, O> {
    #[inline(always)]
    pub fn new(arena: &mut Arena, f: Function<I, O>, input: I) -> Self {
        let input = if size_of::<I>() <= TASK_INLINE_SIZE {
            // store the input data raw
            let mut buf: MaybeUninit<[u8; TASK_INLINE_SIZE]> = MaybeUninit::uninit();
            let buf = unsafe {
                std::ptr::write(buf.as_mut_ptr() as *mut I, input);
                buf.assume_init()
            };
            TaskIO { inline: buf }
        } else {
            TaskIO {
                boxed: ManuallyDrop::new(arena.alloc(input)),
            }
        };
        Self {
            f,
            io: input,
            execute_stolen: Self::execute_stolen,
        }
    }

    #[inline(always)]
    pub fn take_input(&mut self, arena: &mut Arena) -> I {
        let mut io = unsafe { std::ptr::read(&mut self.io as *mut TaskIO<I, O>) };
        if size_of::<I>() <= TASK_INLINE_SIZE {
            unsafe { std::ptr::read(io.inline.as_mut_ptr() as *mut I) }
        } else {
            arena.take(ManuallyDrop::into_inner(unsafe { io.boxed }))
        }
    }
    #[inline(always)]
    pub fn take_output(&mut self, arena: &mut Arena) -> O {
        let io = unsafe { std::ptr::read(&mut self.io as *mut TaskIO<I, O>) };
        let data: ManuallyDrop<(Option<ArenaBox<I>>, Box<O>)> = unsafe { io.output };
        let (abox_opt, obox) = ManuallyDrop::into_inner(data);
        if let Some(abox) = abox_opt {
            // the stolen value may have been copied out of the ArenaBox,
            // depending on what kind of arena we are using, so to be sure
            // not to leak memory when stealing we free it here
            arena.dealloc(abox);
        }
        *obox
    }
    #[inline(always)]
    pub fn execute(mut self, worker: &mut Worker) -> O {
        let ipt = self.take_input(&mut worker.arena);
        (self.f)(worker, ipt)
    }

    fn execute_stolen(task_erased: &mut TypeErased, worker: &mut Worker) {
        let task: &mut Self = unsafe { transmute(task_erased) };
        let mut io = unsafe { std::ptr::read(&mut task.io as *mut TaskIO<I, O>) };
        if size_of::<I>() <= TASK_INLINE_SIZE {
            let ipt = unsafe { std::ptr::read(io.inline.as_mut_ptr() as *mut I) };
            let out = (task.f)(worker, ipt);
            task.io = TaskIO {
                output: ManuallyDrop::new((None, Box::from(out))),
            };
        } else {
            let mut abox: ArenaBox<I> = ManuallyDrop::into_inner(unsafe { io.boxed });
            let ipt = abox.steal();
            let out = (task.f)(worker, ipt);
            task.io = TaskIO {
                output: ManuallyDrop::new((Some(abox), Box::from(out))),
            };
        }
    }

    pub fn erase(self) -> TypeErased {
        unsafe { transmute(self) }
    }
}
pub struct TypeErased {
    _fill: [u8; size_of::<Task<u8, u8>>()],
}
impl TypeErased {
    pub fn unerase<I, O>(self) -> Task<I, O> {
        unsafe { transmute(self) }
    }

    #[inline(always)]
    pub fn execute_stolen(&mut self, worker: &mut Worker) {
        // the generic types don't change the layout of the task struct,
        // so we can safely access the execute_stolen pointer here
        let f: fn(&mut TypeErased, &mut Worker) =
            unsafe { transmute::<&mut Self, &mut Task<(), ()>>(self) }.execute_stolen;
        f(self, worker);
    }
}
