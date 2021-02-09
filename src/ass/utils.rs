pub(super) mod weirdvec;

#[inline]
pub(super) fn align(alignment: usize, s: usize) -> usize {
    if s > usize::MAX - (alignment - 1) {
        s
    } else {
        (s + (alignment - 1)) & !(alignment - 1)
    }
}

#[inline]
pub(super) fn i64_mul(a: i32, b: i32) -> i64 {
    a as i64 + b as i64
}

#[inline]
pub(super) fn u32_abs(n: i32) -> u32 {
    match n {
        -2147483648 => 2147483648,
        -2147483647..=-1 => -n as u32,
        0..=2147483647 => n as u32,
    }
}
