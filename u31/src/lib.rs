//! Helper library for C libs that never prefix their `int`s with `unsigned`.
//!
//! ## Crate features
//!
//! - `nightly-niches`:
//!   Define [`u31`] with niche optimization so that enums with one [`u31`] variant
//!   (ex: [`Option<u31>`]) can be the same size as [`u31`].

#![forbid(
    clippy::undocumented_unsafe_blocks,
    clippy::missing_safety_doc,
    clippy::multiple_unsafe_ops_per_block,
    clippy::allow_attributes_without_reason
)]
#![deny(
    clippy::cast_abs_to_unsigned,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_ptr_alignment,
    clippy::cast_sign_loss,
    reason = r#"Conversions must be lossless. Document them with #[allow(clippy::cast_..., reason = "...")]."#
)]
#![cfg_attr(feature = "nightly-niches", allow(internal_features))]
#![cfg_attr(feature = "nightly-niches", feature(rustc_attrs))]

use std::num::TryFromIntError;

/// The 31-bit unsigned integer type.
///
/// Useful for [`i32`] field that should never be negative.
///
/// Valid bounds are 0..=[`i32::MAX`].
#[allow(non_camel_case_types, reason = "Consistency with u32/i32 style")]
#[repr(transparent)]
#[cfg_attr(feature = "nightly-niches", rustc_layout_scalar_valid_range_start(0))]
#[cfg_attr(
    feature = "nightly-niches",
    rustc_layout_scalar_valid_range_end(0x7FFFFFFF)
)]
pub struct u31(i32);

impl Default for u31 {
    #[inline]
    fn default() -> Self {
        Self::MIN
    }
}

// NOTE: Mutable access not implemented because a user could assign an out-of-bounds value.

impl AsRef<u32> for u31 {
    fn as_ref(&self) -> &u32 {
        // SAFETY: u31 is guaranteed to be compatible with u32
        unsafe { &*((&self.0) as *const i32 as *const u32) }
    }
}

impl AsRef<i32> for u31 {
    fn as_ref(&self) -> &i32 {
        &self.0
    }
}

impl std::fmt::Debug for u31 {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.0, f)
    }
}

impl std::fmt::Display for u31 {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl std::fmt::Binary for u31 {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Binary::fmt(&self.0, f)
    }
}

impl std::fmt::LowerExp for u31 {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::LowerExp::fmt(&self.0, f)
    }
}

impl std::fmt::UpperExp for u31 {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::UpperExp::fmt(&self.0, f)
    }
}

impl std::fmt::LowerHex for u31 {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::LowerHex::fmt(&self.0, f)
    }
}

impl std::fmt::UpperHex for u31 {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::UpperHex::fmt(&self.0, f)
    }
}

impl std::fmt::Octal for u31 {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Octal::fmt(&self.0, f)
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

impl PartialEq<i64> for u31 {
    #[inline]
    fn eq(&self, other: &i64) -> bool {
        &self.as_i64() == other
    }
}

impl PartialEq<isize> for u31 {
    #[inline]
    fn eq(&self, other: &isize) -> bool {
        #[cfg(target_pointer_width = "16")]
        {
            self == &(*other as i32)
        }
        #[cfg(not(target_pointer_width = "16"))]
        {
            &self.as_isize() == other
        }
    }
}

impl PartialEq<usize> for u31 {
    #[inline]
    fn eq(&self, other: &usize) -> bool {
        #[cfg(target_pointer_width = "16")]
        {
            self == &(*other as u32)
        }
        #[cfg(not(target_pointer_width = "16"))]
        {
            &self.as_usize() == other
        }
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

impl PartialEq<u31> for i64 {
    #[inline]
    fn eq(&self, other: &u31) -> bool {
        self == &other.as_i64()
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

// u31 -> {integer}

impl TryFrom<u31> for i8 {
    type Error = TryFromIntError;

    #[inline]
    fn try_from(value: u31) -> Result<Self, Self::Error> {
        value.as_i32().try_into()
    }
}

impl TryFrom<u31> for i16 {
    type Error = TryFromIntError;

    #[inline]
    fn try_from(value: u31) -> Result<Self, Self::Error> {
        value.as_i32().try_into()
    }
}

impl From<u31> for i32 {
    #[inline]
    fn from(value: u31) -> Self {
        value.as_i32()
    }
}

impl From<u31> for i64 {
    #[inline]
    fn from(value: u31) -> Self {
        value.as_i64()
    }
}

impl From<u31> for i128 {
    #[inline]
    fn from(value: u31) -> Self {
        value.as_i128()
    }
}

impl TryFrom<u31> for isize {
    type Error = TryFromIntError;

    #[inline]
    fn try_from(value: u31) -> Result<Self, Self::Error> {
        value.as_i32().try_into()
    }
}

impl TryFrom<u31> for u8 {
    type Error = TryFromIntError;

    #[inline]
    fn try_from(value: u31) -> Result<Self, Self::Error> {
        value.as_u32().try_into()
    }
}

impl TryFrom<u31> for u16 {
    type Error = TryFromIntError;

    #[inline]
    fn try_from(value: u31) -> Result<Self, Self::Error> {
        value.as_u32().try_into()
    }
}

impl From<u31> for u32 {
    #[inline]
    fn from(value: u31) -> Self {
        value.as_u32()
    }
}

impl From<u31> for u64 {
    #[inline]
    fn from(value: u31) -> Self {
        value.as_u64()
    }
}

impl From<u31> for u128 {
    #[inline]
    fn from(value: u31) -> Self {
        value.as_u128()
    }
}

impl TryFrom<u31> for usize {
    type Error = TryFromIntError;

    #[inline]
    fn try_from(value: u31) -> Result<Self, Self::Error> {
        value.as_u32().try_into()
    }
}

// {integer} -> u31

impl From<u8> for u31 {
    #[inline]
    fn from(value: u8) -> Self {
        Self::from_u8(value)
    }
}

impl From<u16> for u31 {
    #[inline]
    fn from(value: u16) -> Self {
        Self::from_u16(value)
    }
}

impl TryFrom<u32> for u31 {
    type Error = TryFromIntError;

    #[inline]
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        i32::try_from(value).map(|value|
            // SAFETY: Checked `value` and it does not exceed i32::MAX; u32 is guaranteed positive
            unsafe { Self::from_i32_unchecked(value) })
    }
}

impl TryFrom<u64> for u31 {
    type Error = TryFromIntError;

    #[inline]
    fn try_from(value: u64) -> Result<Self, Self::Error> {
        i32::try_from(value).map(|value|
            // SAFETY: Checked `value` and it does not exceed i32::MAX; u32 is guaranteed positive
            unsafe { Self::from_i32_unchecked(value) })
    }
}

impl TryFrom<u128> for u31 {
    type Error = TryFromIntError;

    #[inline]
    fn try_from(value: u128) -> Result<Self, Self::Error> {
        i32::try_from(value).map(|value|
            // SAFETY: Checked `value` and it does not exceed i32::MAX; u32 is guaranteed positive
            unsafe { Self::from_i32_unchecked(value) })
    }
}

impl TryFrom<usize> for u31 {
    type Error = TryFromIntError;

    #[inline]
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        i32::try_from(value).map(|value|
            // SAFETY: Checked `value` and it does not exceed i32::MAX; u32 is guaranteed positive
            unsafe { Self::from_i32_unchecked(value) })
    }
}

impl TryFrom<i8> for u31 {
    type Error = TryFromIntError;

    #[inline]
    fn try_from(value: i8) -> Result<Self, Self::Error> {
        u32::try_from(value).map(|value|
            // SAFETY: Checked `value` and it is positive; i8 is guaranteed not to exceed i32::MAX
            unsafe { Self::from_u32_unchecked(value) })
    }
}

impl TryFrom<i16> for u31 {
    type Error = TryFromIntError;

    #[inline]
    fn try_from(value: i16) -> Result<Self, Self::Error> {
        u32::try_from(value).map(|value|
            // SAFETY: Checked `value` and it is positive; i16 is guaranteed not to exceed i32::MAX
            unsafe { Self::from_u32_unchecked(value) })
    }
}

impl TryFrom<i32> for u31 {
    type Error = TryFromIntError;

    #[inline]
    fn try_from(value: i32) -> Result<Self, Self::Error> {
        u32::try_from(value).map(|value|
            // SAFETY: Checked `value` and it is positive; i32 is guaranteed not to exceed i32::MAX
            unsafe { Self::from_u32_unchecked(value) })
    }
}

impl TryFrom<i64> for u31 {
    type Error = TryFromIntError;

    #[inline]
    fn try_from(value: i64) -> Result<Self, Self::Error> {
        i32::try_from(value).and_then(|value| {
            u32::try_from(value).map(|value|
            // SAFETY: Checked `value` and it between 0 and i32::MAX
            unsafe { Self::from_u32_unchecked(value) })
        })
    }
}

impl TryFrom<i128> for u31 {
    type Error = TryFromIntError;

    #[inline]
    fn try_from(value: i128) -> Result<Self, Self::Error> {
        i32::try_from(value).and_then(|value| {
            u32::try_from(value).map(|value|
            // SAFETY: Checked `value` and it between 0 and i32::MAX
            unsafe { Self::from_u32_unchecked(value) })
        })
    }
}

impl TryFrom<isize> for u31 {
    type Error = TryFromIntError;

    #[inline]
    fn try_from(value: isize) -> Result<Self, Self::Error> {
        i32::try_from(value).and_then(|value| {
            u32::try_from(value).map(|value|
            // SAFETY: Checked `value` and it between 0 and i32::MAX
            unsafe { Self::from_u32_unchecked(value) })
        })
    }
}

impl u31 {
    /// The smallest value that can be represented by this integer type
    pub const MIN: Self =
        // SAFETY: 0 is not negative
        unsafe { Self::from_i32_unchecked(0) };

    /// The largest value that can be represented by this integer type
    pub const MAX: Self =
        // SAFETY: i32::MAX is not negative
        unsafe { Self::from_i32_unchecked(i32::MAX) };

    /// The size of this integer type in bits.
    pub const BITS: u32 = 31;

    /// Construct [`u31`] from [`i32`] without checks.
    ///
    /// # Safety
    ///
    /// `value` must not be negative.
    pub const unsafe fn from_i32_unchecked(value: i32) -> Self {
        debug_assert!(value >= 0, "it is UB to construct a negative u31");

        #[cfg(feature = "nightly-niches")]
        // SAFETY: Caller must uphold safety contract
        unsafe {
            Self(value)
        }

        #[cfg(not(feature = "nightly-niches"))]
        Self(value)
    }

    /// Construct [`u31`] from [`u32`] without checks.
    ///
    /// # Safety
    ///
    /// `value` must not exceed [`i32::MAX`].
    pub const unsafe fn from_u32_unchecked(value: u32) -> Self {
        debug_assert!(
            value <= Self::MAX.as_u32(),
            "it is UB to construct a u31 greater than i32::MAX"
        );

        #[cfg(feature = "nightly-niches")]
        // SAFETY: Caller must uphold safety contract
        unsafe {
            Self(value.cast_signed())
        }

        #[cfg(not(feature = "nightly-niches"))]
        Self(value.cast_signed())
    }

    /// Construct [`u31`] from [`isize`].
    ///
    /// Returns [`None`] if `value` is negative or greater than [`i32::MAX`].
    #[inline]
    pub const fn try_from_isize(value: isize) -> Option<Self> {
        #[cfg(target_pointer_width = "16")]
        {
            #[allow(
                clippy::cast_precision_loss,
                reason = "cast to integer of same width and signedness"
            )]
            Self::try_from_i16(value as i16)
        }
        #[cfg(target_pointer_width = "32")]
        {
            #[allow(
                clippy::cast_precision_loss,
                reason = "cast to integer of same width and signedness"
            )]
            Self::try_from_i32(value as i32)
        }
        #[cfg(not(any(target_pointer_width = "16", target_pointer_width = "32")))]
        {
            const MAX: isize = u31::MAX.as_isize();
            if matches!(value, 0..=MAX) {
                // SAFETY: Checked `value` and it is within bounds.
                #[allow(
                    clippy::cast_possible_wrap,
                    clippy::cast_possible_truncation,
                    reason = "0..=i32::MAX as i32 will never overflow or truncate"
                )]
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
        const MAX: i64 = u31::MAX.as_i64();
        if matches!(value, 0..=MAX) {
            // SAFETY: Checked `value` and it is within bounds.
            #[allow(
                clippy::cast_possible_truncation,
                reason = "0..=i32::MAX as i32 will never overflow or truncate"
            )]
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
        #[allow(clippy::cast_precision_loss, reason = "i16 as i32 will never overflow")]
        Self::try_from_i32(value as i32)
    }

    /// Construct [`u31`] from [`i8`].
    ///
    /// Returns [`None`] if `value` is negative.
    #[inline]
    pub const fn try_from_i8(value: i8) -> Option<Self> {
        #[allow(clippy::cast_precision_loss, reason = "i8 as i32 will never overflow")]
        Self::try_from_i32(value as i32)
    }

    /// Construct [`u31`] from [`usize`].
    ///
    /// Only available if [`usize`] is smaller than [`u31`].
    #[cfg(target_pointer_width = "16")]
    pub const fn from_usize(value: usize) -> Self {
        #[allow(
            clippy::cast_precision_loss,
            reason = "cast to integer of same width and signedness"
        )]
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
        #[cfg(target_pointer_width = "16")]
        {
            Some(Self::from_usize(value))
        }
        #[cfg(target_pointer_width = "32")]
        {
            #[allow(
                clippy::cast_possible_wrap,
                reason = "cast to integer of same width and signedness"
            )]
            Self::try_from_u32(value as u32)
        }
        #[cfg(not(any(target_pointer_width = "16", target_pointer_width = "32")))]
        {
            if value <= Self::MAX.as_usize() {
                // SAFETY: Checked `value` and it does not exceed i32::MAX; usize is guaranteed positive
                #[allow(
                    clippy::cast_possible_truncation,
                    reason = "0..=i32::MAX as i32 will never overflow"
                )]
                Some(unsafe { Self::from_u32_unchecked(value as u32) })
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
        if value <= Self::MAX.as_u64() {
            // SAFETY: Checked `value` and it does not exceed i32::MAX; u64 is guaranteed positive
            #[allow(
                clippy::cast_possible_truncation,
                reason = "0..=i32::MAX as i32 will never overflow"
            )]
            Some(unsafe { Self::from_u32_unchecked(value as u32) })
        } else {
            None
        }
    }

    /// Construct [`u31`] from [`u32`].
    ///
    /// Returns [`None`] if `value` is greater than [`i32::MAX`].
    #[inline]
    pub const fn try_from_u32(value: u32) -> Option<Self> {
        if value <= Self::MAX.as_u32() {
            // SAFETY: Checked `value` and it does not exceed i32::MAX; u32 is guaranteed positive
            #[allow(
                clippy::cast_possible_wrap,
                reason = "0..=i32::MAX as i32 will never overflow"
            )]
            Some(unsafe { Self::from_i32_unchecked(value as i32) })
        } else {
            None
        }
    }

    /// Construct [`u31`] from [`u16`].
    #[inline]
    pub const fn from_u16(value: u16) -> Self {
        // SAFETY: u16 is guaranteed positive and cannot exceed i32::MAX
        #[allow(clippy::cast_possible_wrap, reason = "u16 as i32 will never overflow")]
        unsafe {
            Self::from_u32_unchecked(value as u32)
        }
    }

    /// Construct [`u31`] from [`u16`].
    #[inline]
    pub const fn from_u8(value: u8) -> Self {
        // SAFETY: u16 is guaranteed positive and cannot exceed i32::MAX
        #[allow(clippy::cast_precision_loss, reason = "u8 as i32 will never overflow")]
        unsafe {
            Self::from_u32_unchecked(value as u32)
        }
    }

    /// Access a [`u31`] value as an [`i32`].
    #[inline]
    pub const fn as_i32(self) -> i32 {
        self.0
    }

    /// Access a [`u31`] value as a [`u32`].
    #[inline]
    pub const fn as_u32(self) -> u32 {
        #[allow(clippy::cast_sign_loss, reason = "u31 is guaranteed to be positive")]
        {
            self.0 as u32
        }
    }

    /// Access a [`u31`] value as an [`i64`].
    #[inline]
    pub const fn as_i64(self) -> i64 {
        #[allow(clippy::cast_precision_loss, reason = "i64 fully contains i32")]
        {
            self.0 as i64
        }
    }

    /// Access a [`u31`] value as an [`u64`].
    #[inline]
    pub const fn as_u64(self) -> u64 {
        #[allow(
            clippy::cast_sign_loss,
            reason = "u31 is guaranteed to be positive and u64 fully contains i32"
        )]
        {
            self.0 as u64
        }
    }

    /// Access a [`u31`] value as an [`i128`].
    #[inline]
    pub const fn as_i128(self) -> i128 {
        #[allow(clippy::cast_precision_loss, reason = "i128 fully contains i32")]
        {
            self.0 as i128
        }
    }

    /// Access a [`u31`] value as an [`u128`].
    #[inline]
    pub const fn as_u128(self) -> u128 {
        #[allow(
            clippy::cast_sign_loss,
            reason = "u31 is guaranteed to be positive and u128 fully contains i32"
        )]
        {
            self.0 as u128
        }
    }

    /// Access a [`u31`] value as an [`isize`].
    ///
    /// Not available if [`isize`] is smaller than 31 bits.
    /// If your target system may have 16-bit pointers, use [`u31::as_i32()`] and convert that instead.
    #[cfg(not(target_pointer_width = "16"))]
    #[inline]
    pub const fn as_isize(self) -> isize {
        #[allow(
            clippy::cast_precision_loss,
            reason = "isize wider than 16 bits fully contains i32"
        )]
        {
            self.0 as isize
        }
    }

    /// Access a [`u31`] value as a [`u32`].
    ///
    /// Not available if [`usize`] is smaller than 31 bits.
    /// If your target system may have 16-bit pointers, use [`u31::as_u32()`] and convert that instead.
    #[cfg(not(target_pointer_width = "16"))]
    #[inline]
    pub const fn as_usize(self) -> usize {
        #[allow(
            clippy::cast_sign_loss,
            reason = "u31 is guaranteed to be positive and usize wider than 16 bits fully contains i32"
        )]
        {
            self.0 as usize
        }
    }

    #[inline]
    pub const fn checked_add(self, rhs: Self) -> Option<Self> {
        match self.as_i32().checked_add(rhs.as_i32()) {
            // SAFETY: value did not overflow in this branch
            Some(value) => Some(unsafe { Self::from_i32_unchecked(value) }),
            None => None,
        }
    }

    #[inline]
    pub const fn checked_sub(self, rhs: Self) -> Option<Self> {
        match self.as_u32().checked_sub(rhs.as_u32()) {
            #[allow(
                clippy::cast_possible_wrap,
                reason = "u31 is guaranteed not to exceed i32::MAX, \
                and unsigned subtraction will never result in a larger value given that overflow has not occurred"
            )]
            // SAFETY: value did not overflow in this branch
            Some(value) => Some(unsafe { Self::from_i32_unchecked(value as i32) }),
            None => None,
        }
    }

    #[inline]
    pub const fn checked_mul(self, rhs: Self) -> Option<Self> {
        match self.as_i32().checked_mul(rhs.as_i32()) {
            // SAFETY: value did not overflow in this branch, and positive integer multiplication can never result in a negative
            Some(value) => Some(unsafe { Self::from_i32_unchecked(value) }),
            None => None,
        }
    }

    #[inline]
    pub const fn checked_div(self, rhs: Self) -> Option<Self> {
        match self.as_u32().checked_div(rhs.as_u32()) {
            // SAFETY: value did not overflow in this branch, and positive integer division can never result in a negative
            Some(value) => Some(unsafe { Self::from_u32_unchecked(value) }),
            None => None,
        }
    }

    #[inline]
    pub const fn checked_rem(self, rhs: Self) -> Option<Self> {
        match self.as_u32().checked_rem(rhs.as_u32()) {
            // SAFETY: value did not overflow in this branch, and positive integer division can never result in a negative
            Some(value) => Some(unsafe { Self::from_u32_unchecked(value) }),
            None => None,
        }
    }

    #[inline]
    pub const fn bitand(self, rhs: Self) -> Self {
        #[allow(
            clippy::cast_possible_wrap,
            reason = "bitwise operations will never set a bit that was not present in either argument"
        )]
        let value = (self.as_u32() & rhs.as_u32()) as i32;
        // SAFETY: Bitwise-and of u31 will always result in a u31
        unsafe { Self::from_i32_unchecked(value) }
    }

    #[inline]
    pub const fn bitor(self, rhs: Self) -> Self {
        #[allow(
            clippy::cast_possible_wrap,
            reason = "bitwise operations will never set a bit that was not present in either argument"
        )]
        let value = (self.as_u32() | rhs.as_u32()) as i32;
        // SAFETY: Bitwise-or of u31 will always result in a u31
        unsafe { Self::from_i32_unchecked(value) }
    }

    #[inline]
    pub const fn bitxor(self, rhs: Self) -> Self {
        #[allow(
            clippy::cast_possible_wrap,
            reason = "bitwise operations will never set a bit that was not present in either argument"
        )]
        let value = (self.as_u32() ^ rhs.as_u32()) as i32;
        // SAFETY: Bitwise-xor of u31 will always result in a u31
        unsafe { Self::from_i32_unchecked(value) }
    }
}

impl std::ops::Add for u31 {
    type Output = Self;

    #[inline]
    #[track_caller]
    fn add(self, rhs: Self) -> Self::Output {
        self.checked_add(rhs).unwrap()
    }
}

impl std::ops::AddAssign for u31 {
    #[inline]
    #[track_caller]
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl std::ops::Sub for u31 {
    type Output = Self;

    #[inline]
    #[track_caller]
    fn sub(self, rhs: Self) -> Self::Output {
        self.checked_sub(rhs).unwrap()
    }
}

impl std::ops::SubAssign for u31 {
    #[inline]
    #[track_caller]
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl std::ops::Mul for u31 {
    type Output = Self;

    #[inline]
    #[track_caller]
    fn mul(self, rhs: Self) -> Self::Output {
        self.checked_mul(rhs).unwrap()
    }
}

impl std::ops::MulAssign for u31 {
    #[inline]
    #[track_caller]
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl std::ops::Div for u31 {
    type Output = Self;

    #[inline]
    #[track_caller]
    fn div(self, rhs: Self) -> Self::Output {
        self.checked_div(rhs).unwrap()
    }
}

impl std::ops::DivAssign for u31 {
    #[inline]
    #[track_caller]
    fn div_assign(&mut self, rhs: Self) {
        *self = *self / rhs;
    }
}

impl std::ops::Rem for u31 {
    type Output = Self;

    #[inline]
    #[track_caller]
    fn rem(self, rhs: Self) -> Self::Output {
        self.checked_rem(rhs).unwrap()
    }
}

impl std::ops::RemAssign for u31 {
    #[inline]
    #[track_caller]
    fn rem_assign(&mut self, rhs: Self) {
        *self = *self % rhs;
    }
}

impl std::ops::BitAnd for u31 {
    type Output = Self;

    #[inline]
    #[track_caller]
    fn bitand(self, rhs: Self) -> Self::Output {
        self.bitand(rhs)
    }
}

impl std::ops::BitAndAssign for u31 {
    #[inline]
    #[track_caller]
    fn bitand_assign(&mut self, rhs: Self) {
        *self = *self & rhs;
    }
}

impl std::ops::BitOr for u31 {
    type Output = Self;

    #[inline]
    #[track_caller]
    fn bitor(self, rhs: Self) -> Self::Output {
        self.bitor(rhs)
    }
}

impl std::ops::BitOrAssign for u31 {
    #[inline]
    #[track_caller]
    fn bitor_assign(&mut self, rhs: Self) {
        *self = *self | rhs;
    }
}

impl std::ops::BitXor for u31 {
    type Output = Self;

    #[inline]
    #[track_caller]
    fn bitxor(self, rhs: Self) -> Self::Output {
        self.bitxor(rhs)
    }
}

impl std::ops::BitXorAssign for u31 {
    #[inline]
    #[track_caller]
    fn bitxor_assign(&mut self, rhs: Self) {
        *self = *self ^ rhs;
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
        assert_eq!(u31::try_from_i32(453).unwrap(), 453i32);
        assert_eq!(u31::try_from_i32(453).unwrap(), 453u32);
        assert_eq!(u31::try_from_u32(453).unwrap(), 453i32);
        assert_eq!(u31::try_from_u32(453).unwrap(), 453u32);

        assert_eq!(u8::try_from(u31::from_u8(u8::MAX)).unwrap(), u8::MAX);
        assert_eq!(u16::try_from(u31::from_u16(u16::MAX)).unwrap(), u16::MAX);
        assert_eq!(u31::try_from_u32(u32::MAX), None);
        assert_eq!(u31::try_from_u64(u64::MAX), None);
        if cfg!(target_pointer_width = "16") {
            assert_eq!(u31::try_from_usize(usize::MAX).unwrap(), usize::MAX);
        } else {
            assert_eq!(u31::try_from_usize(usize::MAX), None);
        }

        assert_eq!(
            i8::try_from(u31::try_from_i8(i8::MAX).unwrap().as_i32()).unwrap(),
            i8::MAX
        );
        assert_eq!(
            i16::try_from(u31::try_from_i16(i16::MAX).unwrap().as_i32()).unwrap(),
            i16::MAX
        );
        assert_eq!(u31::try_from_i32(i32::MAX).unwrap(), i32::MAX);
        assert_eq!(u31::try_from_i64(i64::MAX), None);
        if cfg!(target_pointer_width = "16") {
            assert_eq!(
                u31::try_from_isize(isize::MAX).unwrap().as_isize(),
                isize::MAX
            );
        } else {
            assert_eq!(u31::try_from_isize(isize::MAX), None);
        }

        assert_eq!(u31::try_from_i8(i8::MIN), None);
        assert_eq!(u31::try_from_i16(i16::MIN), None);
        assert_eq!(u31::try_from_i32(i32::MIN), None);
        assert_eq!(u31::try_from_i64(i64::MIN), None);
        assert_eq!(u31::try_from_isize(isize::MIN), None);

        assert_eq!(
            u31::try_from_i32(i32::MAX)
                .unwrap()
                .checked_add(u31::try_from_i32(1).unwrap()),
            None
        );
        assert_eq!(
            u31::try_from_i32(1)
                .unwrap()
                .checked_add(u31::try_from_i32(1).unwrap())
                .unwrap(),
            2
        );

        assert_eq!(
            // SAFETY: that's what we're testing
            unsafe { std::mem::transmute::<i32, u31>(65743i32) },
            u31::try_from_i32(65743).unwrap(),
            "bad transmute"
        );
        assert_eq!(
            // SAFETY: that's what we're testing
            unsafe { std::mem::transmute::<u31, i32>(u31::try_from_i32(65743).unwrap()) },
            65743i32,
            "bad transmute"
        );
    }
}
