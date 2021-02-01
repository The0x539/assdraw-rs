#[inline]
pub(super) fn align(alignment: usize, s: usize) -> usize {
    if s > usize::MAX - (alignment - 1) {
        s
    } else {
        (s + (alignment - 1)) & !(alignment - 1)
    }
}
