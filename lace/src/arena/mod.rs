#[cfg(not(feature = "unsafe_arena"))]
pub mod safe;
#[cfg(not(feature = "unsafe_arena"))]
pub use safe as picked;

#[cfg(feature = "unsafe_arena")]
pub mod r#unsafe;
#[cfg(feature = "unsafe_arena")]
pub use r#unsafe as picked;

#[cfg(test)]
mod tests {
    use super::picked::*;
    use std::hint::black_box;

    #[test]
    fn smoke() {
        let mut arena = Arena::new();
        let mut x = arena.alloc('x');
        assert_eq!(x.steal(), 'x');
        let y = arena.alloc('y');
        assert_eq!(arena.take(y), 'y');
        let z = arena.alloc('z');
        assert_eq!(arena.take(z), 'z');
        let a = arena.alloc('a');
        assert_eq!(arena.take(a), 'a');
        let b = arena.alloc('b');
        assert_eq!(arena.take(b), 'b');
    }

    #[test]
    fn big_alloc() {
        let mut arena = Arena::new();
        let mut buf = [0 as u8; 12345];
        for i in 0..255 {
            buf[i] = i as u8;
        }
        let ptr = arena.alloc(buf);
        // black_box() to prevent the compiler inlining the alloc-free
        let buf = arena.take(black_box(ptr));
        // check the data integrity
        for i in 0..255 {
            assert_eq!(buf[i], i as u8);
        }
    }

    #[test]
    fn stress() {
        let mut arena = Arena::new();
        struct Big {
            value: usize,
            _fill: [u8; 35],
        }
        let mut small = Vec::new();
        let mut big = Vec::new();
        for _ in 0..10 {
            for i in 0..100000 {
                if i % 5 == 0 {
                    big.push(arena.alloc(Big {
                        value: 456,
                        _fill: [0; 35],
                    }))
                } else {
                    small.push(arena.alloc(123));
                }
            }
            while small.len() + big.len() > 10000 {
                if let Some(ptr) = small.pop() {
                    let x = arena.take(ptr);
                    assert_eq!(x, 123);
                }
                if let Some(ptr) = big.pop() {
                    let x = arena.take(ptr);
                    assert_eq!(x.value, 456);
                }
            }
        }
    }
}
