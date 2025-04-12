use std::{
    alloc::Layout,
    convert::TryFrom,
    marker::PhantomData,
    num::NonZeroUsize,
    ops::{Deref, DerefMut},
    ptr::NonNull,
};
use crate::error::MemAllocError;
use crate::ffi;

/// A trait for providing the deallocate function for a raylib-allocated array.
#[doc(hidden)]
pub trait MemFree<T> {
    /// Release the pointer
    ///
    /// # Guarantees
    /// Specific panics and safety allowances may change depending on implementation, but safety and non-panic **must** be guaranteed in all implementations if **all** of the following conditions are true:
    /// - `ptr` was allocated by the *exact* allocator (generic argument included) intended for this deallocator; typically either a `Load` function or *the* `MemAllocator` implementation attached to the same instance
    /// - `ptr` has not been freed yet
    /// - `count` accurately describes how many instances of `T` are stored in `ptr`
    /// - `count` does not exceed [`i32::MAX`] (some implementations may allow greater, but this is the universal minimum)
    /// - `std::mem::size::<[T; count]>()` does not exceed [`isize::MAX`]
    unsafe fn free(&mut self, ptr: NonNull<T>, count: NonZeroUsize);
}

/// Marker trait for [`MemFree`]/[`MemAlloc`]/[`MemRealloc`] implementations that use their [`Default::default()`] in [`DataBuf::from_raw`]
pub trait GlobalMemFree: Default {}

/// A trait for providing the allocate function for a raylib-allocated array.
///
/// **Note:** This should *only* be implemented if the user is intended to be able to allocate their own resources through this allocator (this is very rare).
#[doc(hidden)]
pub trait MemAlloc<T>: MemFree<T> {
    /// Reserve new memory
    ///
    /// # Guarantees
    /// - The memory must always be aligned
    /// - `count` must accurately describe the number of `T` stored in the return, not the number of bytes
    unsafe fn alloc(&mut self, count: NonZeroUsize) -> Result<NonNull<T>, MemAllocError>;
}

/// A trait for providing the reallocate function for a raylib-allocated array.
///
/// **Note:** This should *only* be implemented if the buffer length is intended to be modifiable by the user (this is very rare).
#[doc(hidden)]
pub trait MemRealloc<T>: MemAlloc<T> {
    unsafe fn realloc(&mut self, ptr: NonNull<T>, old_count: NonZeroUsize, new_count: NonZeroUsize) -> Result<NonNull<T>, MemAllocError>;
}

#[doc(hidden)]
#[derive(Default)]
pub struct RaylibAllocator;
impl<T> MemFree<T> for RaylibAllocator {
    /// Internal memory free
    #[inline]
    unsafe fn free(&mut self, ptr: NonNull<T>, _count: NonZeroUsize) {
        unsafe {
            ffi::MemFree(ptr.as_ptr().cast());
        }
    }
}
impl<T> MemAlloc<T> for RaylibAllocator {
    /// Internal memory allocator
    #[inline]
    unsafe fn alloc(&mut self, count: NonZeroUsize) -> Result<NonNull<T>, MemAllocError> {
        let layout = Layout::array::<T>(count.get())?;
        let size = u32::try_from(layout.size())?;
        NonNull::new(unsafe { ffi::MemAlloc(size) }.cast()).ok_or(MemAllocError::AllocFailed)
    }
}
impl<T> MemRealloc<T> for RaylibAllocator {
    /// Internal memory reallocator
    #[inline]
    unsafe fn realloc(&mut self, ptr: NonNull<T>, _old_count: NonZeroUsize, new_count: NonZeroUsize) -> Result<NonNull<T>, MemAllocError> {
        let layout = Layout::array::<T>(new_count.get())?;
        let size = u32::try_from(layout.size())?;
        NonNull::new(unsafe { ffi::MemRealloc(ptr.as_ptr().cast(), size) }.cast()).ok_or(MemAllocError::AllocFailed)
    }
}
impl GlobalMemFree for RaylibAllocator {}

struct RawDataBuf<T> {
    buf: NonNull<T>,
    count: NonZeroUsize,
    _marker: PhantomData<[T]>,
}
impl<T: std::fmt::Debug> std::fmt::Debug for RawDataBuf<T> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&**self, f)
    }
}
impl<T> Deref for RawDataBuf<T> {
    type Target = [T];

    #[inline]
    fn deref(&self) -> &Self::Target {
        // This is safe because RawDataBuf contents are checked everywhere `buf` can be set.
        unsafe { &*std::ptr::slice_from_raw_parts(self.buf.as_ptr(), self.count.get()) }
    }
}
impl<T> DerefMut for RawDataBuf<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        // This is safe because RawDataBuf contents are checked everywhere `buf` can be set.
        unsafe { &mut *std::ptr::slice_from_raw_parts_mut(self.buf.as_ptr(), self.count.get()) }
    }
}
impl<T> AsRef<[T]> for RawDataBuf<T> {
    #[inline]
    fn as_ref(&self) -> &[T] {
        self.deref()
    }
}
impl<T> AsMut<[T]> for RawDataBuf<T> {
    #[inline]
    fn as_mut(&mut self) -> &mut [T] {
        self.deref_mut()
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
pub struct DataBuf<T, A: MemFree<T> = RaylibAllocator> {
    inner: Option<RawDataBuf<T>>,
    alloc: A,
}
impl<T, A: MemFree<T>> Drop for DataBuf<T, A> {
    fn drop(&mut self) {
        if let Some(inner) = self.inner.take() {
            unsafe {
                self.alloc.free(inner.buf.cast(), inner.count);
            }
        }
    }
}
impl<T: std::fmt::Debug, A: MemFree<T>> std::fmt::Debug for DataBuf<T, A> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self.as_ref(), f)
    }
}
impl<T, A: MemFree<T>> Deref for DataBuf<T, A> {
    type Target = [T];

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.inner.as_ref().map_or(&[], |inner| inner.deref())
    }
}
impl<T, A: MemFree<T>> DerefMut for DataBuf<T, A> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.as_mut().map_or(&mut [], |inner| inner.deref_mut())
    }
}
impl<T, A: MemFree<T>> AsRef<[T]> for DataBuf<T, A> {
    #[inline]
    fn as_ref(&self) -> &[T] {
        self.deref()
    }
}
impl<T, A: MemFree<T>> AsMut<[T]> for DataBuf<T, A> {
    #[inline]
    fn as_mut(&mut self) -> &mut [T] {
        self.deref_mut()
    }
}
impl<T, A: MemFree<T>> DataBuf<T, A> {
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
    /// This method may panic if the buffer is not compatible with a slice (`&mut [T]`).
    pub(crate) fn from_raw_in(buf: *mut T, count: usize, alloc: A) -> Option<Self> {
        if let Some(count) = NonZeroUsize::new(count) {
            NonNull::new(buf).map(|buf| {
                assert!(Layout::array::<T>(count.get()).is_ok(), "DataBuf must be compatible with a slice to be constructed");
                Self { inner: Some(RawDataBuf { buf, count, _marker: PhantomData }), alloc }
            })
        } else {
            Some(Self { inner: None, alloc })
        }
    }

    /// Extract the pointer without freeing it, for the purpose of converting it to another type with its own deallocation (that deallocates the memory as a step).
    ///
    /// Returns [`None`] if the allocation is empty.
    ///
    /// If you're putting this in a drop function, consider making a custom [`MemFree`] instead.
    pub(crate) fn take(mut self) -> Option<(NonNull<T>, NonZeroUsize)> {
        self.inner.take().map(|inner| (inner.buf, inner.count))
    }
}
impl<T, A: MemAlloc<T>> DataBuf<T, A> {
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
    pub fn alloc_in(count: usize, mut alloc: A) -> Result<Self, MemAllocError> {
        let inner = if let Some(count) = NonZeroUsize::new(count) {
            let buf = unsafe { alloc.alloc(count) }?;
            Some(RawDataBuf { buf, count, _marker: PhantomData })
        } else { None };
        Ok(Self { inner, alloc })
    }
}
impl<T, A: MemRealloc<T>> DataBuf<T, A> {
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
    pub fn realloc(&mut self, new_count: usize) -> Result<(), MemAllocError> {
        if let Some(new_count) = NonZeroUsize::new(new_count) {
            if let Some(inner) = &mut self.inner {
                inner.buf = unsafe { self.alloc.realloc(inner.buf, inner.count, new_count) }?;
                inner.count = new_count;
            } else {
                self.inner = Some(RawDataBuf {
                    buf: unsafe { self.alloc.alloc(new_count) }?,
                    count: new_count,
                    _marker: PhantomData,
                })
            }
        } else {
            if let Some(inner) = &mut self.inner {
                unsafe {
                    self.alloc.free(inner.buf, inner.count);
                }
                self.inner = None;
            }
        }
        Ok(())
    }
}
impl<T, A: MemFree<T> + GlobalMemFree> DataBuf<T, A> {
    #[inline]
    pub(crate) fn from_raw(buf: *mut T, count: usize) -> Option<Self> {
        Self::from_raw_in(buf, count, A::default())
    }
}
impl<T, A: MemAlloc<T> + GlobalMemFree> DataBuf<T, A> {
    #[inline]
    pub fn alloc(count: usize) -> Result<Self, MemAllocError> {
        Self::alloc_in(count, A::default())
    }
}
