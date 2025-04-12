use std::{
    alloc::Layout, convert::TryFrom, ffi::c_void, marker::PhantomData, num::NonZeroUsize, ops::{Deref, DerefMut}, ptr::NonNull
};
use crate::error::MemAllocError;
use crate::ffi;

// Owned

/// A trait for providing the deallocate function for a raylib-allocated array.
#[doc(hidden)]
pub trait MemFree {
    type Item;
    /// Release the pointer
    ///
    /// # Guarantees
    /// Specific panics and safety allowances may change depending on implementation, but safety and non-panic **must** be guaranteed in all implementations if **all** of the following conditions are true:
    /// - `ptr` was allocated by the *exact* allocator (generic argument included) intended for this deallocator; typically either a `Load` function or *the* `MemAllocator` implementation attached to the same instance
    /// - `ptr` has not been freed yet
    /// - `count` accurately describes how many instances of `Self::Item` are stored in `ptr`
    /// - `count` does not exceed [`i32::MAX`] (some implementations may allow greater, but this is the universal minimum)
    /// - `std::mem::size::<[Self::Item; count]>()` does not exceed [`isize::MAX`]
    unsafe fn free(&mut self, ptr: NonNull<Self::Item>, count: NonZeroUsize);
}

/// A trait for providing the allocate function for a raylib-allocated array.
///
/// **Note:** This should *only* be implemented if the user is intended to be able to allocate their own resources through this allocator (this is very rare).
#[doc(hidden)]
pub trait MemAlloc: MemFree {
    /// Reserve new memory
    ///
    /// # Guarantees
    /// - The memory must always be aligned
    /// - `count` must accurately describe the number of `T` stored in the return, not the number of bytes
    fn alloc(&mut self, count: NonZeroUsize) -> Result<NonNull<Self::Item>, MemAllocError>;
}

/// A trait for providing the reallocate function for a raylib-allocated array.
///
/// **Note:** This should *only* be implemented if the buffer length is intended to be modifiable by the user (this is very rare).
#[doc(hidden)]
pub trait MemRealloc: MemAlloc {
    unsafe fn realloc(&mut self, ptr: NonNull<Self::Item>, old_count: NonZeroUsize, new_count: NonZeroUsize) -> Result<NonNull<Self::Item>, MemAllocError>;
}

/// Marker trait for [`MemFree`]/[`MemAlloc`]/[`MemRealloc`] implementations that are constructed in [`DataBuf::from_raw`] with [`Default::default()`].
pub trait GlobalMemFree: MemFree + Default {}

#[derive(Default, Clone, Copy)]
struct RaylibInternalAllocator;
impl MemFree for RaylibInternalAllocator {
    type Item = c_void;

    /// Internal memory free
    #[inline]
    unsafe fn free(&mut self, ptr: NonNull<c_void>, _count: NonZeroUsize) {
        unsafe {
            ffi::MemFree(ptr.as_ptr().cast());
        }
    }
}
impl MemAlloc for RaylibInternalAllocator {
    /// Internal memory allocator
    #[inline]
    fn alloc(&mut self, count: NonZeroUsize) -> Result<NonNull<c_void>, MemAllocError> {
        let layout = Layout::array::<c_void>(count.get())?;
        let size = u32::try_from(layout.size())?;
        NonNull::new(unsafe { ffi::MemAlloc(size) }.cast()).ok_or(MemAllocError::AllocFailed)
    }
}
impl MemRealloc for RaylibInternalAllocator {
    /// Internal memory reallocator
    #[inline]
    unsafe fn realloc(&mut self, ptr: NonNull<c_void>, _old_count: NonZeroUsize, new_count: NonZeroUsize) -> Result<NonNull<c_void>, MemAllocError> {
        let layout = Layout::array::<c_void>(new_count.get())?;
        let size = u32::try_from(layout.size())?;
        NonNull::new(unsafe { ffi::MemRealloc(ptr.as_ptr().cast(), size) }.cast()).ok_or(MemAllocError::AllocFailed)
    }
}
impl GlobalMemFree for RaylibInternalAllocator {}

#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct RaylibAllocator<T>(RaylibInternalAllocator, PhantomData<T>);
impl<T> Default for RaylibAllocator<T> {
    fn default() -> Self {
        Self(RaylibInternalAllocator, PhantomData)
    }
}
impl<T> MemFree for RaylibAllocator<T> {
    type Item = T;

    /// Internal memory free
    #[inline]
    unsafe fn free(&mut self, ptr: NonNull<T>, count: NonZeroUsize) {
        unsafe {
            self.0.free(ptr.cast::<c_void>(), count);
        }
    }
}
impl<T> MemAlloc for RaylibAllocator<T> {
    /// Internal memory allocator
    #[inline]
    fn alloc(&mut self, count: NonZeroUsize) -> Result<NonNull<T>, MemAllocError> {
        self.0.alloc(count).map(NonNull::cast::<T>)
    }
}
impl<T> MemRealloc for RaylibAllocator<T> {
    /// Internal memory reallocator
    #[inline]
    unsafe fn realloc(&mut self, ptr: NonNull<T>, old_count: NonZeroUsize, new_count: NonZeroUsize) -> Result<NonNull<T>, MemAllocError> {
        self.0.realloc(ptr.cast::<c_void>(), old_count, new_count).map(NonNull::cast::<T>)
    }
}
impl<T> GlobalMemFree for RaylibAllocator<T> {}

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
/// Automatically releases the memory with [`MemFree::free()`] when dropped.
///
/// Dereference or call [`Self::as_ref()`]/[`Self::as_mut()`] to access the memory as a `&[T]` or `&mut [T]` respectively.
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
pub struct DataBuf<T, A: MemFree<Item = T> = RaylibAllocator<T>> {
    inner: Option<RawDataBuf<T>>,
    alloc: A,
}
impl<T, A: MemFree<Item = T>> Drop for DataBuf<T, A> {
    fn drop(&mut self) {
        if let Some(inner) = self.inner.take() {
            unsafe {
                self.alloc.free(inner.buf.cast(), inner.count);
            }
        }
    }
}
impl<T: std::fmt::Debug, A: MemFree<Item = T>> std::fmt::Debug for DataBuf<T, A> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self.as_ref(), f)
    }
}
impl<T, A: MemFree<Item = T>> Deref for DataBuf<T, A> {
    type Target = [T];

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.inner.as_ref().map_or(&[], |inner| inner.deref())
    }
}
impl<T, A: MemFree<Item = T>> DerefMut for DataBuf<T, A> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.as_mut().map_or(&mut [], |inner| inner.deref_mut())
    }
}
impl<T, A: MemFree<Item = T>> AsRef<[T]> for DataBuf<T, A> {
    #[inline]
    fn as_ref(&self) -> &[T] {
        self.deref()
    }
}
impl<T, A: MemFree<Item = T>> AsMut<[T]> for DataBuf<T, A> {
    #[inline]
    fn as_mut(&mut self) -> &mut [T] {
        self.deref_mut()
    }
}
impl<T, A: MemFree<Item = T>> DataBuf<T, A> {
    /// Wrap an already allocated pointer in a `DataBuf`.
    ///
    /// **Note:** This method is only intended for use with pointers given by Raylib
    /// with the expectation that they will be manually deallocated with [`ffi::MemFree`].
    /// DO NOT use this function to wrap arbitrary pointers or pointers that Raylib will
    /// deallocate itself.
    ///
    /// If the pointer is expected to be conditionally deallocated by Raylib,
    /// (i.e. conditionally passing the buffer to a Raylib function that will certainly deallocatate it)
    /// use [`DataBuf::take`] to unwrap the memory so that `drop` does not automatically free it.
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
impl<T, A: MemAlloc<Item = T>> DataBuf<T, A> {
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
            let buf = alloc.alloc(count)?;
            Some(RawDataBuf { buf, count, _marker: PhantomData })
        } else { None };
        Ok(Self { inner, alloc })
    }
}
impl<T, A: MemRealloc<Item = T>> DataBuf<T, A> {
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
                    buf: self.alloc.alloc(new_count)?,
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
impl<T, A: MemFree<Item = T> + GlobalMemFree> DataBuf<T, A> {
    #[inline]
    pub(crate) fn from_raw(buf: *mut T, count: usize) -> Option<Self> {
        Self::from_raw_in(buf, count, A::default())
    }
}
impl<T, A: MemAlloc<Item = T> + GlobalMemFree> DataBuf<T, A> {
    #[inline]
    pub fn alloc(count: usize) -> Result<Self, MemAllocError> {
        Self::alloc_in(count, A::default())
    }
}
impl<T: Clone, A: MemFree<Item = T>> From<DataBuf<T, A>> for Vec<T> {
    /// Constructs a new [`Vec`] containing a clone of the allocation's contents, unloading the original allocation.
    #[inline]
    fn from(value: DataBuf<T, A>) -> Self {
        Vec::from(value.as_ref())
    }
}
impl<T: Clone, A: MemFree<Item = T>> From<DataBuf<T, A>> for Box<[T]> {
    /// Constructs a new [`Box`] containing a clone of the allocation's contents, unloading the original allocation.
    #[inline]
    fn from(value: DataBuf<T, A>) -> Self {
        Box::from(value.as_ref())
    }
}
impl<T: Copy, A: MemFree<Item = T>, const N: usize> TryFrom<DataBuf<T, A>> for [T; N] {
    type Error = std::array::TryFromSliceError;

    /// Copies the allocation's contents into an array, unloading the original allocation.
    #[inline]
    fn try_from(value: DataBuf<T, A>) -> Result<Self, Self::Error> {
        <[T; N]>::try_from(value.as_ref())
    }
}

// Weak

pub struct WeakWrapper<T, U>(T, PhantomData<U>);
impl<T: std::fmt::Debug, U> std::fmt::Debug for WeakWrapper<T, U> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("WeakBuf").field(&self.0).finish()
    }
}
impl<T: Clone, U> Clone for WeakWrapper<T, U> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), PhantomData)
    }
}
impl<T: Copy, U> Copy for WeakWrapper<T, U> {}
impl<T, U> AsRef<T> for WeakWrapper<T, U> {
    #[inline]
    fn as_ref(&self) -> &T {
        self.deref()
    }
}
impl<T, U> AsMut<T> for WeakWrapper<T, U> {
    #[inline]
    fn as_mut(&mut self) -> &mut T {
        self.deref_mut()
    }
}
impl<T, U> Deref for WeakWrapper<T, U> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<T, U> DerefMut for WeakWrapper<T, U> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl<T, U> WeakWrapper<T, U> {
    /// Take the raw ffi type. Must manually free memory by calling the proper unload function.
    #[inline]
    pub unsafe fn take(self) -> T {
        self.0
    }
    /// Returns the unwrapped raylib-sys object
    #[inline]
    pub fn to_raw(self) -> T {
        self.0
    }
    /// Converts raylib-sys object to a "safe" version. Make sure to call this function from the thread the resource was created.
    #[inline]
    pub unsafe fn from_raw(raw: T) -> Self {
        Self(raw, PhantomData)
    }
    #[inline]
    pub(crate) unsafe fn as_unique(&self) -> &U {
        let p = std::ptr::from_ref::<T>(&self.0).cast::<U>();
        unsafe { &*p }
    }
    #[inline]
    pub(crate) unsafe fn as_unique_mut(&mut self) -> &mut U {
        let p = std::ptr::from_mut::<T>(&mut self.0).cast::<U>();
        unsafe { &mut *p }
    }
}
