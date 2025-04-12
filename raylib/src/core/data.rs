//! Data manipulation functions. Compress and Decompress with DEFLATE
use std::{
    alloc::Layout, ffi::{c_char, CString}, marker::PhantomData, ops::{Deref, DerefMut}, path::Path, ptr::NonNull
};

use crate::{
    error::{AllocationError, CompressionError},
    ffi,
};

#[doc(hidden)]
pub trait MemAllocator: MemDeallocator {
    unsafe fn alloc(layout: Layout) -> Option<NonNull<u8>>;

    #[inline]
    unsafe fn realloc(ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<u8>, NonNull<u8>> {
        let new_ptr = unsafe { Self::alloc(new_layout) };
        if let Some(new_ptr) = new_ptr {
            unsafe {
                std::ptr::copy_nonoverlapping(ptr.as_ptr(), new_ptr.as_ptr(), old_layout.size().min(new_layout.size()));
            }
            Self::free(ptr, old_layout);
            Ok(new_ptr)
        } else {
            Err(ptr)
        }
    }
}

#[doc(hidden)]
pub trait MemDeallocator {
    unsafe fn free(ptr: NonNull<u8>, layout: Layout);
}

#[doc(hidden)]
pub struct RaylibInternalAllocator;
impl MemDeallocator for RaylibInternalAllocator {
    /// Internal memory free
    #[inline]
    unsafe fn free(ptr: NonNull<u8>, _layout: Layout) {
        unsafe {
            ffi::MemFree(ptr.as_ptr().cast());
        }
    }
}
impl MemAllocator for RaylibInternalAllocator {
    /// Internal memory allocator
    #[inline]
    unsafe fn alloc(layout: Layout) -> Option<NonNull<u8>> {
        NonNull::new(unsafe { ffi::MemAlloc(layout.size() as u32) }.cast())
    }

    /// Internal memory reallocator
    #[inline]
    unsafe fn realloc(ptr: NonNull<u8>, _old_layout: Layout, new_layout: Layout) -> Result<NonNull<u8>, NonNull<u8>> {
        NonNull::new(unsafe { ffi::MemRealloc(ptr.as_ptr().cast(), new_layout.size() as u32) }.cast())
            .ok_or(ptr)
    }
}

/// A wrapper acting as an owned buffer for Raylib-allocated memory.
/// Automatically releases the memory with [`ffi::MemFree()`] when dropped.
///
/// Dereference or call `.as_ref()`/`.as_mut()` to access the memory as a `&[T]` or `&mut [T]` respectively.
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
pub struct DataBuf<T: Copy, A: MemDeallocator = RaylibInternalAllocator> {
    buf: NonNull<T>,
    len: usize,
    _alloc: PhantomData<A>,
}
impl<T: Copy + std::fmt::Debug, A: MemDeallocator> std::fmt::Debug for DataBuf<T, A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&**self, f)
    }
}
impl<T: Copy, A: MemDeallocator> Drop for DataBuf<T, A> {
    fn drop(&mut self) {
        unsafe {
            A::free(self.buf.cast(), self.layout());
        }
    }
}
impl<T: Copy, A: MemDeallocator> Deref for DataBuf<T, A> {
    type Target = [T];
    fn deref(&self) -> &Self::Target {
        // This is safe because DataBuf contents are checked everywhere `buf` can be set.
        unsafe { &*std::ptr::slice_from_raw_parts(self.buf.as_ptr(), self.len) }
    }
}
impl<T: Copy, A: MemDeallocator> DerefMut for DataBuf<T, A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // This is safe because DataBuf contents are checked everywhere `buf` can be set.
        unsafe { &mut *std::ptr::slice_from_raw_parts_mut(self.buf.as_ptr(), self.len) }
    }
}
impl<T: Copy, A: MemDeallocator> AsRef<[T]> for DataBuf<T, A> {
    #[inline]
    fn as_ref(&self) -> &[T] {
        self.deref()
    }
}
impl<T: Copy, A: MemDeallocator> AsMut<[T]> for DataBuf<T, A> {
    #[inline]
    fn as_mut(&mut self) -> &mut [T] {
        self.deref_mut()
    }
}
impl<T: Copy, A: MemDeallocator> DataBuf<T, A> {
    #[inline]
    fn layout(&self) -> Layout {
        Layout::array::<T>(self.len).expect("DataBuf layout should always be valid")
    }

    /// Wrap an already allocated pointer in a `DataBuf`.
    ///
    /// **Note:** This method is only intended for use with pointers given by Raylib
    /// with the expectation that they will be manually deallocated with [`ffi::MemFree`].
    /// DO NOT use this function to wrap arbitrary pointers or pointers that Raylib will
    /// deallocate itself.
    ///
    /// If the pointer is expected to be conditionally deallocated by Raylib,
    /// (i.e. conditionally passing the buffer to a Raylib function that will certainly deallocatate it)
    /// use [`DataBuf::leak`] to unwrap the memory so that `drop` does not automatically free it.
    ///
    /// # Returns
    ///
    /// This method returns [`None`] if `buf` is null.
    ///
    /// # Panics
    ///
    /// This method may panic if any of the following are true while `buf` is non-null:
    /// - `count` is less than 1
    /// - `buf` is unaligned
    /// - total bytes exceed [`isize::MAX`]
    pub(crate) fn new(buf: *mut T, count: usize) -> Option<Self> {
        NonNull::new(buf).map(|buf| {
            // Ensure DataBuf can always be dereferenced as a slice.
            assert!(count >= 1, "non-null data should be at least 1 byte");
            assert!(buf.is_aligned(), "DataBuf should be aligned");
            assert!(std::mem::size_of::<T>()
                .checked_mul(count as usize)
                .is_some_and(|total_size| total_size <= (isize::MAX as usize)),
                "total size of DataBuf should not exceed `isize::MAX`");

            Self { buf, len: count as usize, _alloc: PhantomData }
        })
    }

    /// Extract the pointer without freeing it, for the purpose of passing it to a function that will deallocate it manually.
    pub(crate) fn leak(self) -> (NonNull<T>, usize) {
        let buf = self.buf;
        let len = self.len;
        std::mem::forget(self);
        (buf, len)
    }
}

impl<T: Copy, A: MemAllocator> DataBuf<T, A> {
    /// Allocate new memory managed by Raylib
    ///
    /// # Errors
    ///
    /// - "cannot allocate less than 1 element": `count` is less than 1.
    /// - "memory request exceeds unsigned integer maximum": The size of `[T; count]` is greater than [`u32::MAX`].
    /// - "memory request exceeds capacity": [`ffi::MemAlloc`] returned null.
    ///
    /// # Panics
    ///
    /// This method may panic if the pointer returned by [`ffi::MemAlloc`] is unaligned.
    pub fn alloc(count: i32) -> Result<Self, AllocationError> {
        if count >= 1 {
            let count = count as usize;
            match Layout::array::<T>(count) {
                Err(_e) => Err(AllocationError::InvalidLayout), // I would like to display `e` if possible
                Ok(layout) => {
                    let size = layout.size();
                    if size <= u32::MAX as usize {
                        if let Some(buf) = unsafe { A::alloc(layout) }.map(|p| p.cast()) {
                            assert!(buf.is_aligned(), "allocated buffer should always be aligned");
                            Ok(Self { buf, len: count, _alloc: PhantomData })
                        } else { Err(AllocationError::ExceedsCapacity) }
                    } else { Err(AllocationError::ExceedsUIntMax) }
                }
            }
        } else { Err(AllocationError::SubMinSize) }
    }

    /// Reallocate memory already managed by Raylib
    ///
    /// # Errors
    ///
    /// - "cannot allocate less than 1 element": `count` is less than 1.
    /// - "memory request exceeds unsigned integer maximum": The size of `[T; count]` is greater than [`u32::MAX`].
    /// - "memory request exceeds capacity": [`ffi::MemRealloc`] returned null. \
    ///   **Warning:** This represents a risk of double-free if `RL_REALLOC` deallocates regardless of reallocation success,
    ///   because `self` will retain the old pointer and `DataBuf`'s drop implementation will still free it.
    ///
    /// # Panics
    ///
    /// This method may panic if the pointer returned by [`ffi::MemRealloc`] is unaligned.
    pub fn realloc(&mut self, new_count: usize) -> Result<(), AllocationError> {
        if new_count >= 1 {
            let new_count = new_count as usize;
            match Layout::array::<T>(new_count) {
                Err(_e) => Err(AllocationError::InvalidLayout), // I would like to display `e` if possible
                Ok(layout) => {
                    let size = layout.size();
                    if size <= u32::MAX as usize {
                        if let Ok(buf) = unsafe { A::realloc(self.buf.cast(), self.layout(), layout) }.map(|p| p.cast()) {
                            assert!(buf.is_aligned(), "allocated buffer should always be aligned");
                            self.buf = buf;
                            self.len = new_count;
                            Ok(())
                        } else { Err(AllocationError::ExceedsCapacity) }
                    } else { Err(AllocationError::ExceedsUIntMax) }
                }
            }
        } else { Err(AllocationError::SubMinSize) }
    }
}

/// Compress data (DEFLATE algorythm)
/// ```rust
/// use raylib::prelude::*;
/// let data = compress_data(b"11111").unwrap();
/// let expected: &[u8] = &[1, 5, 0, 250, 255, 49, 49, 49, 49, 49];
/// assert_eq!(data.as_ref(), expected);
/// ```
pub fn compress_data(data: &[u8]) -> Result<DataBuf<u8>, CompressionError> {
    let mut out_length: i32 = 0;
    // CompressData doesn't actually modify the data, but the header is wrong
    let buffer = {
        unsafe { ffi::CompressData(data.as_ptr() as *mut _, data.len() as i32, &mut out_length) }
    };
    DataBuf::new(buffer, out_length as usize)
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
pub fn decompress_data(data: &[u8]) -> Result<DataBuf<u8>, CompressionError> {
    #[cfg(debug_assertions)]
    println!("{:?}", data.len());

    let mut out_length: i32 = 0;
    // CompressData doesn't actually modify the data, but the header is wrong
    let buffer = {
        unsafe { ffi::DecompressData(data.as_ptr() as *mut _, data.len() as i32, &mut out_length) }
    };
    DataBuf::new(buffer, out_length as usize)
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
