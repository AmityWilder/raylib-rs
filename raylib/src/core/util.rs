use std::{borrow::Cow, ffi::{CStr, CString, FromBytesWithNulError, NulError}, path::{Path, PathBuf}};

/// An error indicating that an interior nul byte was found.
///
/// A trailing nul is appended where necessary by [`IntoCStr`], so `NotNulTerminated`
/// is not needed as a variant for this error.
///
/// See [`NulError`] and [`FromBytesWithNulError`] for more info.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct IntoCStrNulError {
    pub position: usize,
}

impl From<NulError> for IntoCStrNulError {
    fn from(value: NulError) -> Self {
        Self {
            position: value.nul_position(),
        }
    }
}

impl std::fmt::Display for IntoCStrNulError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "data provided contains an interior nul byte at pos {}", self.position)
    }
}

impl std::error::Error for IntoCStrNulError {}

/// Used to convert strings into ffi-compatible [`CStr`]s.
///
/// Because strings in C must be nul-terminated, but strings in Rust are not,
/// an allocation may be required to copy the Rust string into a new [`CString`]
/// that can then be referenced as a [`CStr`]. However, in the case of [`CStr`]
/// itself and nul-terminated `[u8]`s, this allocation can be avoided altogether.
///
/// This trait provides specializations for multiple common string types to
/// minimize unnecessary allocations.
///
/// [`IntoCStr`] always produces a type that implements [`IntoCStr`], **often as
/// a no-op**. So it is safe and performant to call `into_cstr()` and pass the
/// (non-erroneous) return to another [`IntoCStr`] argument.
///
/// **Note to users of raylib-rs:**
///
/// If the string you are passing to a [`IntoCStr`] argument is already a literal,
/// consider making it a [`CStr`] literal by placing a `c` before the open quote.
///
/// e.g.
/// ```no_run
/// d.draw_text( "Hello World!", ...) // before
/// d.draw_text(c"Hello World!", ...) // after
/// ```
///
/// This small change will eliminate a runtime allocation that would have been
/// used just to store the already-compiletime-constant text.
///
/// Additionally, if you are passing a [`format!`]'d string by reference and don't
/// need to use it anywhere else, consider passing the owned string directly so
/// that the original allocation can possibly be used instead of allocating a second
/// one to append nul into.
///
/// e.g.
/// ```no_run
/// d.draw_text(&format!("I have {} bananas", n), ...) // before
/// d.draw_text( format!("I have {} bananas", n), ...) // after
/// ```
pub trait IntoCStr: Sized {
    /// The [`CStr`]-like type produced by `into_cstr()` if there are no errors.
    type Output: IntoCStrNoOp;

    /// Convert string to a type that can be referenced as a nul-terminated [`CStr`].
    ///
    /// Returns [`IntoCStrNulError`] if an interior byte is 0.
    ///
    /// **Warning for callers**
    ///
    /// Some Implementations return an owned [`CString`] instead of a borrowed [`CStr`]. It
    /// is the caller's responsibility to ensure that the return outlives the function it
    /// is being passed to. Calling `as_ptr` on an owned [`CString`] that hasn't been stored
    /// to a variable will drop the owned allocation immediately after `as_ptr` returns,
    /// resulting in a dangling pointer.
    ///
    /// See the documentation of [`CStr::as_ptr`] for more info.
    ///
    /// # Example
    ///
    /// ```
    /// # use crate::core::util::ToCStr;
    /// # use std::ffi::c_char;
    ///
    /// unsafe fn cstr_fn(c_str: *const c_char) {
    ///     // ...
    /// }
    ///
    /// fn wrapper_fn(text: impl ToCStr) {
    ///     // into_cstr must be called outside of cstr_fn's mouth
    ///     let c_text = text.into_cstr();
    ///     unsafe {
    ///         cstr_fn(c_text.as_ptr())
    ///     }
    /// }
    /// ```
    #[must_use]
    fn into_cstr(self) -> Result<Self::Output, IntoCStrNulError>;
}

/// Marker trait for types that can be converted to a [`CStr`]-compatible type both
/// trivially and infallibly.
///
/// Types that implement this trait are guaranteed to return `self` when `into_cstr`
/// is called, and can therefore be converted to `const char*` without conversion.
pub trait IntoCStrNoOp: std::ops::Deref<Target = CStr> {
    #[inline]
    fn as_ptr(&self) -> *const std::ffi::c_char {
        CStr::as_ptr(self)
    }
}

/// No-op. Returns `self` unconditionally.
impl<T: IntoCStrNoOp> IntoCStr for T {
    type Output = Self;

    #[inline]
    fn into_cstr(self) -> Result<T, IntoCStrNulError> {
        Ok(self)
    }
}

impl IntoCStrNoOp for &CStr {}
impl IntoCStrNoOp for CString {}
impl IntoCStrNoOp for Cow<'_, CStr> {}

/// Returns `self` as a `&CStr` unconditionally.
impl<'a> IntoCStr for &'a CString {
    type Output = &'a CStr;

    #[inline]
    fn into_cstr(self) -> Result<&'a CStr, IntoCStrNulError> {
        Ok(self.as_c_str())
    }
}

/// Tries to convert the slice to a [`CStr`] without allocating.
/// If the string is not nul-terminated, a [`CString`] is allocated.
impl<'a> IntoCStr for &'a [u8] {
    type Output = Cow<'a, CStr>;

    #[inline]
    fn into_cstr(self) -> Result<Cow<'a, CStr>, IntoCStrNulError> {
        use FromBytesWithNulError::*;
        match CStr::from_bytes_with_nul(self) {
            Ok(s) => Ok(Cow::Borrowed(s)),
            Err(InteriorNul { position }) => Err(IntoCStrNulError { position }),
            Err(NotNulTerminated) => match CString::new(self) {
                Ok(s) => Ok(Cow::Owned(s)),
                Err(e) => Err(e.into()),
            },
        }
    }
}

/// Appends the [`Vec<u8>`] with nul (if there isn't a trailing nul) and converts it to a [`CString`].
/// No allocation is needed if `self` has capacity for an additional element.
impl IntoCStr for Vec<u8> {
    type Output = CString;

    #[inline]
    fn into_cstr(self) -> Result<CString, IntoCStrNulError> {
        CString::new(self)
            .map_err(IntoCStrNulError::from)
    }
}

/// [`str`] is not nul-terminated. Always allocates.
impl IntoCStr for &str {
    type Output = CString;

    #[inline]
    fn into_cstr(self) -> Result<CString, IntoCStrNulError> {
        CString::new(self)
            .map_err(IntoCStrNulError::from)
    }
}

/// Appends the [`String`] with nul (if there isn't a trailing nul) and converts it to a [`CString`].
/// No allocation is needed if `self` has capacity for an additional element.
impl IntoCStr for String {
    type Output = <Vec<u8> as IntoCStr>::Output;

    #[inline]
    fn into_cstr(self) -> Result<CString, IntoCStrNulError> {
        self.into_bytes().into_cstr()
    }
}

/// [`Path`] is not nul-terminated. Always allocates.
/// This is infallible on unix platforms, because unix [`Path`]s cannot contain interior nuls.
///
/// TODO: Needs testing for WTF-8 strings
impl IntoCStr for &Path {
    type Output = CString;

    #[inline]
    fn into_cstr(self) -> Result<CString, IntoCStrNulError> {
        CString::new(self.as_os_str().as_encoded_bytes())
            .map_err(IntoCStrNulError::from)
    }
}

/// Appends the [`PathBuf`] with nul and converts it to a [`CString`].
/// No allocation is needed if `self` has capacity for an additional element.
/// This is infallible on unix platforms, because unix [`Path`]s cannot contain interior nuls.
///
/// TODO: Needs testing for WTF-8 strings
impl IntoCStr for PathBuf {
    type Output = CString;

    #[inline]
    fn into_cstr(self) -> Result<CString, IntoCStrNulError> {
        CString::new(self.into_os_string().into_encoded_bytes())
            .map_err(IntoCStrNulError::from)
    }
}
