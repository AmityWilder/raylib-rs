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
/// # Example
///
/// ```no_run
/// # use crate::core::util::ToCStr;
/// unsafe extern "C" {
///     fn ffi_fn(text: *const std::ffi::c_char);
/// }
///
/// fn call_ffi_fn(string: impl ToCStr) {
///     let c_string = string.to_cstr();
///     unsafe {
///         ffi_fn(c_string.as_ref().as_ptr());
///     }
/// }
/// ```
///
/// **Note to users of raylib-rs:**
///
/// If the string you are passing to a [`ToCStr`] argument is already a literal,
/// consider making it a [`CStr`] literal by placing a `c` before the open quote.
///
/// (e.g. `"Hello World!"` -> `c"Hello World!"`)
///
/// This small change will eliminate a runtime allocation that would have been
/// used just to store the already-compiletime-constant text.
pub trait IntoCStr: Sized {
    type Output: AsRef<CStr>;

    /// Convert string to a type that can be referenced as a nul-terminated [`CStr`].
    ///
    /// Returns [`None`] if an interior byte is 0. See [`std::ffi::NulError`] for more info.
    ///
    /// **Warning for callers**
    ///
    /// Some Implementations of `to_cstr` return an owned [`CString`] instead of a borrowed
    /// [`CStr`]. It is the caller's responsibility to ensure that the return outlives the
    /// function it is being passed to. Calling `as_ptr` on an owned [`CString`] that hasn't
    /// been stored to a variable will drop the owned allocation immediately after `as_ptr`
    /// returns, resulting in a dangling pointer.
    ///
    /// See the documentation of [`CStr::as_ptr`] for more info.
    #[must_use]
    fn into_cstr(self) -> Result<Self::Output, IntoCStrNulError>;
}

/// No-op. Returns `self` unconditionally.
impl<'a> IntoCStr for &'a CStr {
    type Output = &'a CStr;

    #[inline]
    fn into_cstr(self) -> Result<&'a CStr, IntoCStrNulError> {
        Ok(self)
    }
}

/// Tries to convert the slice to a [`CStr`] without allocating.
/// If the string is not nul-terminated, a [`CString`] is allocated.
impl<'a> IntoCStr for &'a [u8] {
    type Output = Cow<'a, CStr>;

    #[inline]
    fn into_cstr(self) -> Result<Cow<'a, CStr>, IntoCStrNulError> {
        match CStr::from_bytes_with_nul(self) {
            Ok(s) => Ok(Cow::Borrowed(s)),
            Err(FromBytesWithNulError::InteriorNul { position }) => Err(IntoCStrNulError { position }),
            Err(FromBytesWithNulError::NotNulTerminated) => match CString::new(self) {
                Ok(s) => Ok(Cow::Owned(s)),
                Err(e) => Err(IntoCStrNulError { position: e.nul_position() }),
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
