use super::{task_body, REAL};

fn rec_matmul(
    a: &[REAL],
    b: &[REAL],
    c: usize, // secretly a *mut REAL
    m: usize,
    n: usize,
    p: usize,
    ld: usize,
    add: bool,
) {
    if (m + n + p) <= 64 {
        task_body(a, b, c as *mut REAL, m, n, p, ld, add);
    } else if m >= n && n >= p {
        let m1 = m >> 1;
        rayon::join(
            || rec_matmul(a, b, c, m1, n, p, ld, add),
            || {
                rec_matmul(
                    &a[m1 * ld..],
                    b,
                    c.wrapping_add(m1 * ld),
                    m - m1,
                    n,
                    p,
                    ld,
                    add,
                )
            },
        );
    } else if n >= m && n >= p {
        let n1 = n >> 1;
        rec_matmul(a, b, c, m, n1, p, ld, add);
        rec_matmul(&a[n1..], &b[n1 * ld..], c, m, n - n1, p, ld, true);
    } else {
        let p1 = p >> 1;
        rayon::join(
            || rec_matmul(a, b, c, m, n, p1, ld, add),
            || rec_matmul(a, &b[p1..], c.wrapping_add(p1), m, n, p - p1, ld, add),
        );
    }
}

pub fn run(n: usize, a: &Vec<REAL>, b: &Vec<REAL>, c: &mut Vec<REAL>) {
    let c_ptr = c as &mut [REAL] as *mut [REAL] as *mut REAL as usize;
    rec_matmul(a, b, c_ptr, n, n, n, n, false);
}
