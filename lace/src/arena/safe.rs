pub struct ArenaBox<T> {
    inner: Option<Box<T>>,
}
impl<T> ArenaBox<T> {
    #[inline(always)]
    pub fn steal(&mut self) -> T {
        *self.inner.take().expect("ArenaBox Value Taken Already")
    }

    #[cfg(feature = "chase_lev")]
    #[inline(always)]
    pub fn into_mut_ptr(self) -> *mut T {
        Box::into_raw(self.inner.unwrap())
    }
    #[cfg(feature = "chase_lev")]
    #[inline(always)]
    pub fn from_mut_ptr(ptr: *mut T) -> Self {
        ArenaBox {
            inner: Some(unsafe { Box::from_raw(ptr) }),
        }
    }
}

pub struct Arena {}
impl Arena {
    pub fn new() -> Self {
        Self {}
    }
    #[inline(always)]
    pub fn alloc<T>(&mut self, value: T) -> ArenaBox<T> {
        ArenaBox {
            inner: Some(Box::new(value)),
        }
    }
    #[inline(always)]
    pub fn dealloc<T>(&mut self, _: ArenaBox<T>) {
        // dropping the ArenaBox will drop the inner Box
    }
    #[inline(always)]
    pub fn take<T>(&mut self, ptr: ArenaBox<T>) -> T {
        *ptr.inner.unwrap()
    }
}
