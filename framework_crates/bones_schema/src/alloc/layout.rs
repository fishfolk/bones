//! Utilities for working with [`Layout`].

use std::alloc::{Layout, LayoutError};

/// Extension trait for the [`Layout`] type that copies useful nightly functions so that we can use
/// them on stable.
pub trait LayoutExt {
    /// Creates a layout describing the record for `n` instances of
    /// `self`, with a suitable amount of padding between each to
    /// ensure that each instance is given its requested size and
    /// alignment. On success, returns `(k, offs)` where `k` is the
    /// layout of the array and `offs` is the distance between the start
    /// of each element in the array.
    ///
    /// On arithmetic overflow, returns `LayoutError`.
    fn repeat(&self, n: usize) -> Result<(Layout, usize), LayoutError>;
    // Returns the amount of padding we must insert after `self`
    /// to ensure that the following address will satisfy `align`
    /// (measured in bytes).
    ///
    /// e.g., if `self.size()` is 9, then `self.padding_needed_for(4)`
    /// returns 3, because that is the minimum number of bytes of
    /// padding required to get a 4-aligned address (assuming that the
    /// corresponding memory block starts at a 4-aligned address).
    ///
    /// The return value of this function has no meaning if `align` is
    /// not a power-of-two.
    ///
    /// Note that the utility of the returned value requires `align`
    /// to be less than or equal to the alignment of the starting
    /// address for the whole allocated block of memory. One way to
    /// satisfy this constraint is to ensure `align <= self.align()`.
    fn padding_needed_for(&self, align: usize) -> usize;
}

// Hack to construct a [`LayoutError`] which cannot be constructed directly.
const LAYOUT_ERR: LayoutError = if let Err(e) = Layout::from_size_align(0, 0) {
    e
} else {
    unreachable!()
};

impl LayoutExt for Layout {
    #[inline]
    fn padding_needed_for(&self, align: usize) -> usize {
        let len = self.size();

        // Rounded up value is:
        //   len_rounded_up = (len + align - 1) & !(align - 1);
        // and then we return the padding difference: `len_rounded_up - len`.
        //
        // We use modular arithmetic throughout:
        //
        // 1. align is guaranteed to be > 0, so align - 1 is always
        //    valid.
        //
        // 2. `len + align - 1` can overflow by at most `align - 1`,
        //    so the &-mask with `!(align - 1)` will ensure that in the
        //    case of overflow, `len_rounded_up` will itself be 0.
        //    Thus the returned padding, when added to `len`, yields 0,
        //    which trivially satisfies the alignment `align`.
        //
        // (Of course, attempts to allocate blocks of memory whose
        // size and padding overflow in the above manner should cause
        // the allocator to yield an error anyway.)

        let len_rounded_up = len.wrapping_add(align).wrapping_sub(1) & !align.wrapping_sub(1);
        len_rounded_up.wrapping_sub(len)
    }

    #[inline]
    fn repeat(&self, n: usize) -> Result<(Self, usize), LayoutError> {
        // This cannot overflow. Quoting from the invariant of Layout:
        // > `size`, when rounded up to the nearest multiple of `align`,
        // > must not overflow isize (i.e., the rounded value must be
        // > less than or equal to `isize::MAX`)
        let padded_size = self.size() + Layout::padding_needed_for(self, self.align());
        let alloc_size = padded_size.checked_mul(n).ok_or(LAYOUT_ERR)?;

        // The safe constructor is called here to enforce the isize size limit.
        let layout = Layout::from_size_align(alloc_size, self.align())?;
        Ok((layout, padded_size))
    }
}
