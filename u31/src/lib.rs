//! Helper library for C libs that never prefix their `int`s with `unsigned`.
//!
//! ## Crate features
//!
//! - `nightly-niches`:
//!   Define [`u31`] with niche optimization so that enums with one [`u31`] variant
//!   (ex: [`Option<u31>`]) can be the same size as [`u31`].

#![forbid(clippy::undocumented_unsafe_blocks, clippy::missing_safety_doc)]
#![deny(clippy::as_conversions, reason = r#"`as` conversions must be documented with #[allow(clippy::as_conversions, reason = "...")]."#)]

#![cfg_attr(feature = "nightly-niches", allow(internal_features))]
#![cfg_attr(feature = "nightly-niches", feature(rustc_attrs))]

/// The 31-bit unsigned integer type.
///
/// Useful for [`i32`] field that should never be negative.
///
/// Valid bounds are 0..=[`i32::MAX`].
#[allow(non_camel_case_types)]
#[repr(transparent)]
#[cfg_attr(feature = "nightly-niches", rustc_layout_scalar_valid_range_start(0))]
#[cfg_attr(feature = "nightly-niches", rustc_layout_scalar_valid_range_end(0x7FFFFFFF))]
pub struct u31(i32);

impl Default for u31 {
    #[inline]
    fn default() -> Self {
        Self::ZERO
    }
}

impl std::fmt::Debug for u31 {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_i32().fmt(f)
    }
}

impl Clone for u31 {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}
impl Copy for u31 {}

impl PartialEq for u31 {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl Eq for u31 {}

impl PartialEq<u32> for u31 {
    #[inline]
    fn eq(&self, other: &u32) -> bool {
        &self.as_u32() == other
    }
}

impl PartialEq<i32> for u31 {
    #[inline]
    fn eq(&self, other: &i32) -> bool {
        &self.as_i32() == other
    }
}

impl PartialEq<u31> for u32 {
    #[inline]
    fn eq(&self, other: &u31) -> bool {
        self == &other.as_u32()
    }
}

impl PartialEq<u31> for i32 {
    #[inline]
    fn eq(&self, other: &u31) -> bool {
        self == &other.as_i32()
    }
}

impl Ord for u31 {
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl PartialOrd for u31 {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialOrd<u32> for u31 {
    #[inline]
    fn partial_cmp(&self, other: &u32) -> Option<std::cmp::Ordering> {
        self.as_u32().partial_cmp(other)
    }
}

impl PartialOrd<i32> for u31 {
    #[inline]
    fn partial_cmp(&self, other: &i32) -> Option<std::cmp::Ordering> {
        self.as_i32().partial_cmp(other)
    }
}

impl PartialOrd<u31> for u32 {
    #[inline]
    fn partial_cmp(&self, other: &u31) -> Option<std::cmp::Ordering> {
        self.partial_cmp(&other.as_u32())
    }
}

impl PartialOrd<u31> for i32 {
    #[inline]
    fn partial_cmp(&self, other: &u31) -> Option<std::cmp::Ordering> {
        self.partial_cmp(&other.0)
    }
}

impl From<u31> for i32 {
    #[inline]
    fn from(value: u31) -> Self {
        value.as_i32()
    }
}

impl From<u31> for u32 {
    #[inline]
    fn from(value: u31) -> Self {
        value.as_u32()
    }
}

impl TryFrom<i32> for u31 {
    type Error = std::num::TryFromIntError;

    #[inline]
    fn try_from(value: i32) -> Result<Self, Self::Error> {
        u32::try_from(value).map(|_|
            // SAFETY: Checked `value` and it does not exceed i32::MAX; u32 is guaranteed positive
            unsafe { Self::from_i32_unchecked(value) }
        )
    }
}

impl TryFrom<u32> for u31 {
    type Error = std::num::TryFromIntError;

    #[inline]
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        i32::try_from(value).map(|value|
            // SAFETY: Checked `value` and it is positive; i32 is guaranteed not to exceed i32::MAX
            unsafe { Self::from_i32_unchecked(value) }
        )
    }
}

impl u31 {
    #[allow(unused_unsafe, reason = "necessary when using niches")]
    pub const ZERO: Self =
        // SAFETY: 0 is not negative and does not exceed i32::MAX
        unsafe { Self(0) };

    /// Construct [`u31`] from [`i32`] without checks.
    ///
    /// # Safety
    ///
    /// `value` must not be negative.
    pub const unsafe fn from_i32_unchecked(value: i32) -> Self {
        #[cfg(feature = "nightly-niches")]
        // SAFETY: Caller must uphold safety contract
        unsafe { Self(value) }

        #[cfg(not(feature = "nightly-niches"))]
        Self(value)
    }

    /// Construct [`u31`] from [`isize`].
    ///
    /// Returns [`None`] if `value` is negative or greater than [`i32::MAX`].
    #[inline]
    pub const fn try_from_isize(value: isize) -> Option<Self> {
        #[cfg(target_pointer_width = "16")] {
            #[allow(clippy::as_conversions, reason = "cast to integer of same width and signedness")]
            Self::try_from_i16(value as i16)
        }
        #[cfg(target_pointer_width = "32")] {
            #[allow(clippy::as_conversions, reason = "cast to integer of same width and signedness")]
            Self::try_from_i32(value as i32)
        }
        #[cfg(not(any(target_pointer_width = "16", target_pointer_width = "32")))] {
            #[allow(clippy::as_conversions, reason = "isize wider than 16 bits fully contains i32")]
            const MAX: isize = i32::MAX as isize;
            if matches!(value, 0..=MAX) {
                // SAFETY: Checked `value` and it is within bounds.
                #[allow(clippy::as_conversions, reason = "0..=i32::MAX as i32 will never overflow")]
                Some(unsafe { Self::from_i32_unchecked(value as i32) })
            } else {
                None
            }
        }
    }

    /// Construct [`u31`] from [`i64`].
    ///
    /// Returns [`None`] if `value` is negative or greater than [`i32::MAX`].
    #[inline]
    pub const fn try_from_i64(value: i64) -> Option<Self> {
        #[allow(clippy::as_conversions, reason = "i64 fully contains i32")]
        const MAX: i64 = i32::MAX as i64;
        if matches!(value, 0..=MAX) {
            // SAFETY: Checked `value` and it is within bounds.
            #[allow(clippy::as_conversions, reason = "0..=i32::MAX as i32 will never overflow")]
            Some(unsafe { Self::from_i32_unchecked(value as i32) })
        } else {
            None
        }
    }

    /// Construct [`u31`] from [`i32`].
    ///
    /// Returns [`None`] if `value` is negative.
    #[inline]
    pub const fn try_from_i32(value: i32) -> Option<Self> {
        if value >= 0 {
            // SAFETY: Checked `value` and it is positive; i32 is guaranteed not to exceed i32::MAX
            Some(unsafe { Self::from_i32_unchecked(value) })
        } else {
            None
        }
    }

    /// Construct [`u31`] from [`i16`].
    ///
    /// Returns [`None`] if `value` is negative.
    #[inline]
    pub const fn try_from_i16(value: i16) -> Option<Self> {
        #[allow(clippy::as_conversions, reason = "i16 as i32 will never overflow")]
        Self::try_from_i32(value as i32)
    }

    /// Construct [`u31`] from [`i8`].
    ///
    /// Returns [`None`] if `value` is negative.
    #[inline]
    pub const fn try_from_i8(value: i8) -> Option<Self> {
        #[allow(clippy::as_conversions, reason = "i8 as i32 will never overflow")]
        Self::try_from_i32(value as i32)
    }

    /// Construct [`u31`] from [`usize`].
    ///
    /// Only available if [`usize`] is smaller than [`u31`].
    #[cfg(target_pointer_width = "16")]
    pub const fn from_usize(value: usize) -> Self {
        #[allow(clippy::as_conversions, reason = "cast to integer of same width and signedness")]
        Self::from_u16(value as u16)
    }

    /// Construct [`u31`] from [`usize`].
    ///
    /// Returns [`None`] if `value` is negative or greater than [`i32::MAX`].
    ///
    /// Infallible on target systems with 16-bit pointers.
    /// Consider using [`u31::from_usize`] to represent this behavior if your
    /// `target_pointer_width` is always 16 bits.
    #[inline]
    pub const fn try_from_usize(value: usize) -> Option<Self> {
        #[cfg(target_pointer_width = "16")] {
            Some(Self::from_usize(value))
        }
        #[cfg(target_pointer_width = "32")] {
            #[allow(clippy::as_conversions, reason = "cast to integer of same width and signedness")]
            Self::try_from_u32(value as u32)
        }
        #[cfg(not(any(target_pointer_width = "16", target_pointer_width = "32")))] {
            #[allow(clippy::as_conversions, reason = "i32::MAX is positive and usize wider than 16 bits fully contains i32")]
            if value <= i32::MAX as usize {
                // SAFETY: Checked `value` and it does not exceed i32::MAX; usize is guaranteed positive
                #[allow(clippy::as_conversions, reason = "0..=i32::MAX as i32 will never overflow")]
                Some(unsafe { Self::from_i32_unchecked(value as i32) })
            } else {
                None
            }
        }
    }

    /// Construct [`u31`] from [`u32`].
    ///
    /// Returns [`None`] if `value` is greater than [`i32::MAX`].
    #[inline]
    pub const fn try_from_u64(value: u64) -> Option<Self> {
        #[allow(clippy::as_conversions, reason = "i32::MAX is positive and u64 fully contains i32")]
        if value <= i32::MAX as u64 {
            // SAFETY: Checked `value` and it does not exceed i32::MAX; u64 is guaranteed positive
            #[allow(clippy::as_conversions, reason = "0..=i32::MAX as i32 will never overflow")]
            Some(unsafe { Self::from_i32_unchecked(value as i32) })
        } else {
            None
        }
    }

    /// Construct [`u31`] from [`u32`].
    ///
    /// Returns [`None`] if `value` is greater than [`i32::MAX`].
    #[inline]
    pub const fn try_from_u32(value: u32) -> Option<Self> {
        #[allow(clippy::as_conversions, reason = "i32::MAX is positive and u32 fully contains i32")]
        if value <= i32::MAX as u32 {
            // SAFETY: Checked `value` and it does not exceed i32::MAX; u32 is guaranteed positive
            #[allow(clippy::as_conversions, reason = "0..=i32::MAX as i32 will never overflow")]
            Some(unsafe { Self::from_i32_unchecked(value as i32) })
        } else {
            None
        }
    }

    /// Construct [`u31`] from [`u16`].
    #[inline]
    pub const fn from_u16(value: u16) -> Self {
        // SAFETY: u16 is guaranteed positive and cannot exceed i32::MAX
        #[allow(clippy::as_conversions, reason = "u16 as i32 will never overflow")]
        unsafe { Self::from_i32_unchecked(value as i32) }
    }

    /// Construct [`u31`] from [`u16`].
    #[inline]
    pub const fn from_u8(value: u8) -> Self {
        // SAFETY: u16 is guaranteed positive and cannot exceed i32::MAX
        #[allow(clippy::as_conversions, reason = "u8 as i32 will never overflow")]
        unsafe { Self::from_i32_unchecked(value as i32) }
    }

    /// Access a [`u31`] value as an [`i32`].
    #[inline]
    pub const fn as_i32(self) -> i32 {
        self.0
    }

    /// Access a [`u31`] value as a [`u32`].
    #[inline]
    pub const fn as_u32(self) -> u32 {
        self.0.cast_unsigned()
    }

    /// Access a [`u31`] value as an [`i64`].
    #[inline]
    pub const fn as_i64(self) -> i64 {
        #[allow(clippy::as_conversions, reason = "i64 fully contains i32")] {
            self.0 as i64
        }
    }

    /// Access a [`u31`] value as an [`isize`].
    ///
    /// Not available if [`isize`] is smaller than [`i32`].
    /// If your target system may have 16-bit pointers, use [`u31::as_i32()`] and convert that instead.
    #[cfg(not(target_pointer_width = "16"))]
    #[inline]
    pub const fn as_isize(self) -> isize {
        #[allow(clippy::as_conversions, reason = "isize greater than 16 bits fully contains i32")] {
            self.0 as isize
        }
    }

    /// Access a [`u31`] value as a [`u32`].
    #[cfg(not(target_pointer_width = "16"))]
    #[inline]
    pub const fn as_usize(self) -> usize {
        #[allow(clippy::as_conversions, reason = "usize greater than 16 bits fully contains u32")] {
            self.0 as usize
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test0() {
        assert_eq!(u31::try_from_u32(5).unwrap(), 5);
        assert_eq!(u31::try_from_i32(5).unwrap(), 5);
        assert_eq!(u31::try_from_i32(-1), None);
        assert_eq!(u31::try_from_u32(i32::MAX as u32 + 1), None);
        assert_eq!(u31::try_from_u32(u32::MAX), None);
        assert_eq!(u31::try_from_u32(0x80000000), None);
        assert_eq!(u31::try_from_i32(453).unwrap().as_i32(), 453i32);
        assert_eq!(u31::try_from_i32(453).unwrap().as_u32(), 453u32);
        assert_eq!(u31::try_from_u32(453).unwrap().as_i32(), 453i32);
        assert_eq!(u31::try_from_u32(453).unwrap().as_u32(), 453u32);

        assert_eq!(u31::try_from_u64(u64::MAX), None);
    }
}
