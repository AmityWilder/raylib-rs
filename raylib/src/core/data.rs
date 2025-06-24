//! Data manipulation functions. Compress and Decompress with DEFLATE
#![forbid(clippy::undocumented_unsafe_blocks, clippy::missing_safety_doc, clippy::missing_panics_doc)]
#![warn(missing_docs, clippy::missing_docs_in_private_items)]
use std::{ffi::{c_char, c_void, CString}, marker::PhantomData, mem::MaybeUninit, ops::{Deref, DerefMut}, path::Path, ptr::NonNull};
use crate::{ffi, error::{AllocationError, CompressionError}};

pub(crate) trait TryIntoUsize {
    fn try_into_usize(self) -> Option<usize>;
}

impl TryIntoUsize for u8 {
    #[inline]
    fn try_into_usize(self) -> Option<usize> {
        Some(self.into())
    }
}
impl TryIntoUsize for u16 {
    #[inline]
    fn try_into_usize(self) -> Option<usize> {
        Some(self.into())
    }
}
impl TryIntoUsize for u32 {
    #[inline]
    fn try_into_usize(self) -> Option<usize> {
        self.try_into().ok()
    }
}
impl TryIntoUsize for u64 {
    #[inline]
    fn try_into_usize(self) -> Option<usize> {
        self.try_into().ok()
    }
}
impl TryIntoUsize for u128 {
    #[inline]
    fn try_into_usize(self) -> Option<usize> {
        self.try_into().ok()
    }
}
impl TryIntoUsize for usize {
    #[inline]
    fn try_into_usize(self) -> Option<usize> {
        Some(self)
    }
}
impl TryIntoUsize for i8 {
    #[inline]
    fn try_into_usize(self) -> Option<usize> {
        self.try_into().ok()
    }
}
impl TryIntoUsize for i16 {
    #[inline]
    fn try_into_usize(self) -> Option<usize> {
        self.try_into().ok()
    }
}
impl TryIntoUsize for i32 {
    #[inline]
    fn try_into_usize(self) -> Option<usize> {
        self.try_into().ok()
    }
}
impl TryIntoUsize for i64 {
    #[inline]
    fn try_into_usize(self) -> Option<usize> {
        self.try_into().ok()
    }
}
impl TryIntoUsize for i128 {
    #[inline]
    fn try_into_usize(self) -> Option<usize> {
        self.try_into().ok()
    }
}
impl TryIntoUsize for isize {
    #[inline]
    fn try_into_usize(self) -> Option<usize> {
        self.try_into().ok()
    }
}

/// Provide 'unload' implementation for a loaded resource.
pub trait Unloader {
    /// The type of the resource to be unloaded.
    type Type: ?Sized;

    /// Unload the resource.
    ///
    /// # Safety
    /// - `data` must have been loaded with the correct allocator for this unloader.
    unsafe fn unload(&mut self, data: NonNull<Self::Type>);
}

/// Raylib default memory management. Frees with [`ffi::MemFree`] (`RL_FREE`).
pub struct MemManaged<T: ?Sized>(PhantomData<T>);

impl<T: ?Sized> Unloader for MemManaged<T> {
    type Type = T;

    /// # Safety
    /// - `data` must have been allocated with [`ffi::MemAlloc`] or `RL_MALLOC`.
    #[inline]
    unsafe fn unload(&mut self, data: NonNull<T>) {
        // SAFETY: Data is non-null and allocated with MemAlloc
        unsafe {
            ffi::MemFree(data.as_ptr().cast::<c_void>());
        }
    }
}

/// An owned buffer for Raylib-allocated memory.
/// Automatically unloads the resource with [`Deallocator::dealloc()`] when dropped.
///
/// # Example
/// ```
/// use raylib::prelude::*;
/// let buf: DataBuf<u8> = compress_data(b"11111").unwrap();
/// // Use this how you used to use the return of `compress_data()`.
/// // It will live until `buf` goes out of scope or gets dropped.
/// let data: &[u8] = buf.as_ref();
/// let expected: &[u8] = &[1, 5, 0, 250, 255, 49, 49, 49, 49, 49];
/// assert_eq!(data, expected);
/// ```
#[derive(Debug)]
pub struct DataBuf<T: ?Sized, A: Unloader<Type = T> = MemManaged<T>> {
    buf: NonNull<T>,
    _marker: PhantomData<T>,
    manager: A,
}

impl<T: ?Sized, A: Unloader<Type = T>> Drop for DataBuf<T, A> {
    #[inline]
    fn drop(&mut self) {
        // SAFETY: unloader given upon construction
        unsafe {
            self.manager.unload(self.buf);
        }
    }
}

impl<T: ?Sized, A: Unloader<Type = T>> Deref for DataBuf<T, A> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { self.buf.as_ref() }
    }
}

impl<T: ?Sized, A: Unloader<Type = T>> DerefMut for DataBuf<T, A> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.buf.as_mut() }
    }
}

impl<T: ?Sized, A: Unloader<Type = T>> AsRef<T> for DataBuf<T, A> {
    #[inline]
    fn as_ref(&self) -> &T {
        self.deref()
    }
}

impl<T: ?Sized, A: Unloader<Type = T>> AsMut<T> for DataBuf<T, A> {
    #[inline]
    fn as_mut(&mut self) -> &mut T {
        self.deref_mut()
    }
}

impl<T> DataBuf<[T], MemManaged<[T]>> {
    /// Allocate a Raylib-owned buffer with `count` elements.
    pub fn new(count: usize) -> Result<Self, AllocationError> {
        let byte_count = std::mem::size_of::<T>().checked_mul(count)
            .and_then(|n| n.try_into().ok())
            .ok_or(AllocationError::BadSize)?;

        // SAFETY:
        // - MemAlloc has no preconditions
        let data = unsafe { ffi::MemAlloc(byte_count) }.cast::<T>();
        // SAFETY:
        // - just allocated `count` elements of `data` with `MemAlloc`
        unsafe { Self::slice_from_raw(data, count) }
            .ok_or(AllocationError::OutOfMemory)
    }

    /// Mark a Raylib-given pointer as an owned slice with len `count`.
    ///
    /// Returns [`None`] if `data` is null.
    ///
    /// # Safety
    /// - if `data` is non-null, it must be valid
    /// - `count` must accurately describe the number of elements in `data`, if `data` is non-null
    ///   - `count` is only observed if `data` is non-null; it is safe to pass an invalid count as long as `data` is null
    /// - non-null `data` must be allocated with [`ffi::MemAlloc`] or `RL_MALLOC`
    #[inline]
    pub(crate) unsafe fn slice_from_raw(data: *mut T, count: impl TryIntoUsize) -> Option<Self> {
        unsafe { Self::slice_from_raw_in(data, count, MemManaged(PhantomData)) }
    }
}

impl<T> DataBuf<T, MemManaged<T>> {
    /// Allocate a Raylib-owned buffer.
    pub fn new() -> Result<Self, AllocationError> {
        let byte_count = std::mem::size_of::<T>()
            .try_into().ok()
            .ok_or(AllocationError::BadSize)?;

        // SAFETY:
        // - MemAlloc has no preconditions
        let data = unsafe { ffi::MemAlloc(byte_count) }.cast::<T>();
        // SAFETY:
        // - just allocated `data` with `MemAlloc`
        unsafe { Self::from_raw(data) }
            .ok_or(AllocationError::OutOfMemory)
    }

    /// Mark a Raylib-given pointer as an owned buffer.
    ///
    /// Returns [`None`] if `data` is null.
    ///
    /// # Safety
    /// - if `data` is non-null, it must be valid
    /// - non-null `data` must be allocated with [`ffi::MemAlloc`] or `RL_MALLOC`
    #[inline]
    pub(crate) unsafe fn from_raw(data: *mut T) -> Option<Self> {
        unsafe { Self::from_raw_in(data, MemManaged(PhantomData)) }
    }
}

impl<T, A: Unloader<Type = [T]>> DataBuf<[T], A> {
    /// Mark a Raylib-given pointer as an owned slice with len `count`.
    ///
    /// Returns [`None`] if `data` is null.
    ///
    /// # Safety
    /// - if `data` is non-null, it must be valid
    /// - `count` must accurately describe the number of elements in `data`, if `data` is non-null
    ///   - `count` is only observed if `data` is non-null; it is safe to pass an invalid count as long as `data` is null
    /// - `manager` must implement the correct unload function for `data`
    #[inline]
    pub(crate) unsafe fn slice_from_raw_in(data: *mut T, count: impl TryIntoUsize, manager: A) -> Option<Self> {
        NonNull::new(data)
            .map(|buf| Self {
                buf: NonNull::slice_from_raw_parts(buf, count.try_into_usize().unwrap()),
                _marker: PhantomData,
                manager,
            })
    }
}

impl<T: ?Sized, A: Unloader<Type = T>> DataBuf<T, A> {
    /// Mark a Raylib-given pointer as an owned buffer.
    ///
    /// Returns [`None`] if `data` is null.
    ///
    /// # Safety
    /// - if `data` is non-null, it must be valid
    /// - `manager` must implement the correct unload function for `data`
    #[inline]
    pub(crate) unsafe fn from_raw_in(data: *mut T, manager: A) -> Option<Self> {
        NonNull::new(data)
            .map(|buf| Self {
                buf,
                _marker: PhantomData,
                manager,
            })
    }
}

/// Compress data (DEFLATE algorythm)
/// ```rust
/// use raylib::prelude::*;
/// let data = compress_data(b"11111").unwrap();
/// let expected: &[u8] = &[1, 5, 0, 250, 255, 49, 49, 49, 49, 49];
/// assert_eq!(data.as_ref(), expected);
/// ```
pub fn compress_data(data: &[u8]) -> Result<DataBuf<[u8]>, CompressionError> {
    let mut out_length = MaybeUninit::uninit();
    let buffer = {
        // SAFETY:
        // - CompressData doesn't actually modify the data, but the header is wrong
        // - `data` and `out_length` are non-null
        // - `dataSize` is accurate to `data`'s size
        // - CompressData does not read `out_length` until it is assigned to
        unsafe { ffi::CompressData(data.as_ptr() as *mut _, data.len() as i32, out_length.as_mut_ptr()) }
    };
    // SAFETY:
    // - `buffer` is allocated with `RL_MALLOC`
    // - `out_length` accurately describes `buffer`'s len when `buffer` is non-null
    unsafe { DataBuf::slice_from_raw(buffer, out_length.assume_init()) }
        .ok_or_else(|| CompressionError::CompressionFailed)
}

/// Decompress data (DEFLATE algorythm)
/// ```rust
/// use raylib::prelude::*;
/// let input: &[u8] = &[1, 5, 0, 250, 255, 49, 49, 49, 49, 49];
/// let expected: &[u8] = b"11111";
/// let data = decompress_data(input).unwrap();
/// assert_eq!(data.as_ref(), expected);
/// ```
pub fn decompress_data(data: &[u8]) -> Result<DataBuf<[u8]>, CompressionError> {
    #[cfg(debug_assertions)]
    println!("{:?}", data.len());

    let mut out_length = MaybeUninit::uninit();
    let buffer = {
        // SAFETY:
        // - DecompressData doesn't actually modify the data, but the header is wrong
        // - `data` and `out_length` are non-null
        // - `dataSize` is accurate to `data`'s size
        // - DecompressData does not read `out_length` until it is assigned to
        unsafe { ffi::DecompressData(data.as_ptr() as *mut _, data.len() as i32, out_length.as_mut_ptr()) }
    };
    // SAFETY:
    // - `buffer` is allocated with `RL_MALLOC`
    // - `out_length` accurately describes `buffer`'s len when `buffer` is non-null
    unsafe { DataBuf::slice_from_raw(buffer, out_length.assume_init()) }
        .ok_or_else(|| CompressionError::CompressionFailed)
}

#[cfg(unix)]
fn path_to_bytes<P: AsRef<Path>>(path: P) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;
    path.as_ref().as_os_str().as_bytes().to_vec()
}

#[cfg(not(unix))]
fn path_to_bytes<P: AsRef<Path>>(path: P) -> Vec<u8> {
    path.as_ref().to_string_lossy().to_string().into_bytes()
}

/// Export data to code (.h), returns true on success
pub fn export_data_as_code(data: &[u8], file_name: impl AsRef<Path>) -> bool {
    let c_str = CString::new(path_to_bytes(file_name)).unwrap();

    unsafe { ffi::ExportDataAsCode(data.as_ptr(), data.len() as i32, c_str.as_ptr()) }
}

/// Encode data to Base64 string
pub fn encode_data_base64(data: &[u8]) -> Vec<c_char> {
    let mut output_size = 0;
    let bytes =
        unsafe { ffi::EncodeDataBase64(data.as_ptr(), data.len() as i32, &mut output_size) };

    let s = unsafe { std::slice::from_raw_parts(bytes, output_size as usize) };
    if s.contains(&0) {
        // Work around a bug in Rust's from_raw_parts function
        let mut keep = true;
        let b: Vec<c_char> = s
            .iter()
            .filter(|f| {
                if **f == 0 {
                    keep = false;
                }
                keep
            })
            .map(|f| *f)
            .collect();
        b
    } else {
        s.to_vec()
    }
}

/// Decode Base64 data
pub fn decode_data_base64(data: &[u8]) -> Vec<u8> {
    let mut output_size = 0;

    let bytes = unsafe { ffi::DecodeDataBase64(data.as_ptr(), &mut output_size) };

    let s = unsafe { std::slice::from_raw_parts(bytes, output_size as usize) };
    if s.contains(&0) {
        // Work around a bug in Rust's from_raw_parts function
        let mut keep = true;
        let b: Vec<u8> = s
            .iter()
            .filter(|f| {
                if **f == 0 {
                    keep = false;
                }
                keep
            })
            .map(|f| *f)
            .collect();
        b
    } else {
        s.to_vec()
    }
}
