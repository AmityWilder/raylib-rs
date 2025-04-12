macro_rules! make_thin_wrapper {
    ($(#[$attrs:meta])* $name:ident, $t:ty, $dropfunc:expr) => {
        make_thin_wrapper!($(#[$attrs])* $name, $t, $dropfunc, true);
    };
    ($(#[$attrs:meta])* $name:ident, $t:ty, $dropfunc:expr, false) => {
        $(#[$attrs])*
        #[repr(transparent)]
        #[derive(Debug)]
        pub struct $name(pub(crate) $t);

        impl_wrapper!($name, $t, $dropfunc, 0);
        gen_from_raw_wrapper!($name, $t, $dropfunc, 0);
    };
    ($(#[$attrs:meta])* $name:ident, $t:ty, $dropfunc:expr, true) => {
        $(#[$attrs])*
        #[repr(transparent)]
        #[derive(Debug)]
        pub struct $name(pub(crate) $t);

        impl_wrapper!($name, $t, $dropfunc, 0);
        deref_impl_wrapper!($name, $t, $dropfunc, 0);
        gen_from_raw_wrapper!($name, $t, $dropfunc, 0);
    };
}

macro_rules! make_thin_wrapper_lifetime {
    ($(#[$attrs:meta])* $name:ident, $t1:ty, $t2:ty, $dropfunc:expr) => {
        make_thin_wrapper_lifetime!($name, $t1, $t2, $dropfunc, true);
    };
    ($(#[$attrs:meta])* $name:ident, $t1:ty, $t2:ty,$dropfunc:expr, false) => {
        #[derive(Debug)]
        pub struct $name<'a>(pub(crate) $t1, &'a $t2);

        impl_wrapper!($name, $t1, $dropfunc, 0);
    };
    ($(#[$attrs:meta])* $name:ident, $t1:ty, $t2:ty, $dropfunc:expr, true) => {
        #[derive(Debug)]
        pub struct $name<'a>(pub(crate) $t1, &'a $t2);

        impl_wrapper!($name<'a>, $t1, $dropfunc, 0);
        deref_impl_wrapper!($name<'a>, $t1, $dropfunc, 0);
    };
}

macro_rules! impl_wrapper {
    ($name:ident$(<$lifetime:tt>)?, $t:ty, $dropfunc:expr, $rawfield:tt) => {
        impl$(<$lifetime>)? $name$(<$lifetime>)? {
            /// Take the raw ffi type. Must manually free memory by calling the proper unload function
            pub unsafe fn unwrap(self) -> $t {
                let inner = self.$rawfield;
                std::mem::forget(self);
                inner
            }
        }

        impl$(<$lifetime>)? Drop for $name$(<$lifetime>)? {
            #[allow(unused_unsafe)]
            fn drop(&mut self) {
                unsafe {
                    ($dropfunc)(self.$rawfield);
                }
            }
        }


    };
}

macro_rules! gen_from_raw_wrapper {
    ($name:ident$(<$lifetime:tt>)?, $t:ty, $dropfunc:expr, $rawfield:tt) => {
        impl$(<$lifetime>)? $name$(<$lifetime>)? {
            /// returns the unwrapped raylib-sys object
            pub fn to_raw(self) -> $t {
                let raw = self.$rawfield;
                std::mem::forget(self);
                raw
            }

            /// converts raylib-sys object to a "safe"
            /// version. Make sure to call this function
            /// from the thread the resource was created.
            pub unsafe fn from_raw(raw: $t) -> Self {
                Self(raw)
            }
        }
    };
}

macro_rules! deref_impl_wrapper {
    ($name:ident$(<$lifetime:tt>)?, $t:ty, $dropfunc:expr, $rawfield:tt) => {
        impl$(<$lifetime>)? std::convert::AsRef<$t> for $name$(<$lifetime>)? {
            fn as_ref(&self) -> &$t {
                &self.$rawfield
            }
        }

        impl$(<$lifetime>)? std::convert::AsMut<$t> for $name$(<$lifetime>)? {
            fn as_mut(&mut self) -> &mut $t {
                &mut self.$rawfield
            }
        }

        impl$(<$lifetime>)? std::ops::Deref for $name$(<$lifetime>)? {
            type Target = $t;
            #[inline]
            fn deref(&self) -> &Self::Target {
                &self.$rawfield
            }
        }

        impl$(<$lifetime>)? std::ops::DerefMut for $name$(<$lifetime>)? {
            #[inline]
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.$rawfield
            }
        }
    };
}

macro_rules! make_data_buffer {
    ($(#[$attrs:meta])* $Name:ident, [$T:ty], $Allocator:ident, $(#[$fn_attrs:meta])* (&mut $self:ident, $ptr:ident, $count:ident) => $dealloc:expr $(,)?) => {
        #[doc(hidden)]
        #[derive(Default)]
        pub struct $Allocator;
        impl $crate::databuf::MemFree for $Allocator {
            type Item = $T;

            #[inline]
            $(#[$fn_attrs])*
            unsafe fn free(&mut $self, $ptr: std::ptr::NonNull<$T>, $count: std::num::NonZeroUsize) {
                $dealloc
            }
        }
        impl $crate::databuf::GlobalMemFree for $Allocator {}
        make_data_buffer!($(#[$attrs])* $Name, [$T], $Allocator);
    };
    ($(#[$attrs:meta])* $Name:ident, [$T:ty]$(, $Allocator:ty)? $(,)?) => {
        $(#[$attrs])*
        pub type $Name = $crate::databuf::DataBuf<$T$(, $Allocator)?>;
    };
}

// macro_rules! make_wrapper {
//     ($(#[$attrs:meta])* $Name:ident, $Raw:ty, $dropfn:expr, $(#[$weak_attrs:meta])* $Weak:ident) => {
//         $(#[$attrs])*
//         pub struct $Name($Raw);
//         impl Drop for $Name {
//             #[allow(unused_unsafe)]
//             fn drop(&mut self) {
//                 unsafe {
//                     ($dropfn)(self.0)
//                 }
//             }
//         }
//         $(#[$weak_attrs])*
//         pub type $Weak = $crate::databuf::WeakWrapper<$Raw, $Name>;
//         impl AsRef<$Name> for $Weak {
//             fn as_ref(&self) -> &$Name {
//                 unsafe { self.as_unique() }
//             }
//         }
//         impl AsMut<$Name> for $Weak {
//             fn as_mut(&mut self) -> &mut $Name {
//                 unsafe { self.as_unique_mut() }
//             }
//         }
//         impl $Name {
//             pub unsafe fn make_weak(self) -> $Weak {
//                 $Weak::from_raw(std::mem::ManuallyDrop::new(self).0)
//             }
//         }
//     };
// }
