use std::mem::{replace, size_of, ManuallyDrop, MaybeUninit};
// LCM of all the bin sizes, so no memory is wasted at the edge of a buffer
const BUFFER_SIZE: usize = 1536;

pub struct ArenaBox<T> {
    ptr: *mut T,
}
impl<T> ArenaBox<T> {
    #[inline(always)]
    pub fn steal(&mut self) -> T {
        unsafe { std::ptr::read(self.ptr) }
    }

    #[cfg(feature = "chase_lev")]
    #[inline(always)]
    pub fn into_mut_ptr(self) -> *mut T {
        self.ptr
    }
    #[cfg(feature = "chase_lev")]
    #[inline(always)]
    pub fn from_mut_ptr(ptr: *mut T) -> Self {
        Self { ptr }
    }
}

struct BufferList {
    buffer: [u8; BUFFER_SIZE],
    previous: Option<Box<BufferList>>,
}
impl BufferList {
    fn new() -> Self {
        Self {
            // initialize with zeroes, so that reading any slot will
            // yield "0": the end of the free list. Strictly speaking
            // we'd only need to have usize 0 at each *mut T boundary,
            // but this works fine.
            buffer: [0; BUFFER_SIZE],
            previous: None,
        }
    }
    #[inline(always)]
    fn start(&self) -> usize {
        self.buffer.as_ptr() as usize
    }
    #[inline(always)]
    fn end(&self) -> usize {
        self.buffer.as_ptr().wrapping_add(BUFFER_SIZE) as *const () as usize
    }
}

const N_BINS: usize = 12;
pub struct Arena {
    // don't immediately initialize each bin slot: most of them will probably
    // never be used, so always initializing wastes a lot of memory
    uninit: [bool; N_BINS],
    // NOTE: the minimal bin size should fit a usize, this code assumes that's 8*sizeof(u8)
    // bin sizes: 8,16,24,32,48,64,96,128,192,256,384,512,768
    // anything larger gets heap-allocated
    bins: ManuallyDrop<[Box<BufferList>; N_BINS]>,
    // the next free slot (pointer), zero if the bin list must be extended
    head: [usize; N_BINS],
}
impl Drop for Arena {
    fn drop(&mut self) {
        for i in 0..N_BINS {
            if !self.uninit[i] {
                // take ownership of the boxed BufferList
                let mut tmp: Box<BufferList> = unsafe {
                    #[allow(invalid_value)]
                    MaybeUninit::uninit().assume_init()
                };
                std::mem::swap(&mut self.bins[i], &mut tmp);
                drop(tmp);
            }
        }
    }
}
impl Arena {
    pub fn new() -> Self {
        Self {
            uninit: [true; N_BINS],
            bins: ManuallyDrop::new(unsafe {
                // the memory is managed manually
                #[allow(invalid_value)]
                MaybeUninit::uninit().assume_init()
            }),
            head: [0; N_BINS],
        }
    }
    #[inline(always)]
    fn bin<T>() -> Option<(usize, usize)> {
        Some(if size_of::<T>() <= 8 {
            (0, 8)
        } else if size_of::<T>() <= 16 {
            (1, 16)
        } else if size_of::<T>() <= 24 {
            (2, 24)
        } else if size_of::<T>() <= 32 {
            (3, 32)
        } else if size_of::<T>() <= 48 {
            (4, 48)
        } else if size_of::<T>() <= 64 {
            (5, 64)
        } else if size_of::<T>() <= 96 {
            (6, 96)
        } else if size_of::<T>() <= 128 {
            (7, 128)
        } else if size_of::<T>() <= 192 {
            (8, 192)
        } else if size_of::<T>() <= 256 {
            (9, 256)
        } else if size_of::<T>() <= 384 {
            (9, 384)
        } else if size_of::<T>() <= 512 {
            (10, 512)
        } else if size_of::<T>() <= 768 {
            (11, 768)
        } else {
            return None;
        })
    }

    #[cold]
    fn init_bin(&mut self, bin: usize) {
        self.uninit[bin] = false;
        unsafe {
            std::ptr::write(
                &mut self.bins[bin] as *mut Box<BufferList>,
                Box::new(BufferList::new()),
            );
        }
    }
    #[cold]
    fn extend_bin(&mut self, bin: usize) -> usize {
        if self.uninit[bin] {
            self.init_bin(bin);
        } else {
            let new_buffer = Box::new(BufferList::new());
            let old_buffer = replace(&mut self.bins[bin], new_buffer);
            self.bins[bin].previous = Some(old_buffer);
        }
        self.bins[bin].start()
    }
    #[cold]
    #[inline(always)]
    fn alloc_bump(&mut self, head: usize, bin: usize, size: usize) -> usize {
        // the buffer is filling up
        if head + 2 * size <= self.bins[bin].end() {
            head + size
        } else {
            // we are about to fill the last bit of space
            // in the current buffer
            0
        }
    }
    #[inline(always)]
    pub fn alloc<T>(&mut self, value: T) -> ArenaBox<T> {
        let Some((bin, size)) = Self::bin::<T>() else {
            // fall back on the heap allocator
            return ArenaBox {
                ptr: Box::into_raw(Box::new(value)),
            };
        };
        let head = self.head[bin];
        let head = if head == 0 {
            // no free slot
            self.extend_bin(bin)
        } else {
            head
        };
        // find the next free slot in the bin, and move the head
        let next: usize = unsafe { std::ptr::read(head as *mut usize) };
        // move the head to the next free slot
        self.head[bin] = if next == 0 {
            self.alloc_bump(head, bin, size)
        } else {
            // move to the next slot in the free list
            next
        };
        unsafe { std::ptr::write(head as *mut T, value) }
        ArenaBox {
            ptr: head as *mut T,
        }
    }
    #[inline(always)]
    pub fn dealloc<T>(&mut self, ptr: ArenaBox<T>) {
        let Some((bin, _)) = Self::bin::<T>() else {
            // fall back on heap allocator
            drop(unsafe { Box::from_raw(ptr.ptr) });
            return;
        };
        let ptr = ptr.ptr as *mut usize;
        // extend the free list
        unsafe { std::ptr::write(ptr, self.head[bin]) };
        self.head[bin] = ptr as usize;
    }
    #[inline(always)]
    pub fn take<T>(&mut self, ptr: ArenaBox<T>) -> T {
        let value = unsafe { std::ptr::read(ptr.ptr) };
        self.dealloc(ptr);
        value
    }
}
