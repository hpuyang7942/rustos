/// Align `addr` downwards to the nearest multiple of `align`.
///
/// The returned usize is always <= `addr.`
///
/// # Panics
///
/// Panics if `align` is not a power of 2.
pub fn align_down(addr: usize, align: usize) -> usize {
    if !align.is_power_of_two() {
        panic!("align is not a power of 2!")
    }
    addr & !(align-1)

    // let trimmed = addr & (align - 1);
    // addr - trimmed
}

/// Align `addr` upwards to the nearest multiple of `align`.
///
/// The returned `usize` is always >= `addr.`
///
/// # Panics
///
/// Panics if `align` is not a power of 2
/// or aligning up overflows the address.
pub fn align_up(addr: usize, align: usize) -> usize {
    align_down(addr.saturating_add(align - 1), align)
}

pub fn prev_power_of_two(num: usize) -> usize {
    1 << (8 * (::core::mem::size_of::<usize>()) - num.leading_zeros() as usize - 1)
}