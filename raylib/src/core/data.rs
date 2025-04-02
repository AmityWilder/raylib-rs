//! Data manipulation functions. Compress and Decompress with DEFLATE
use std::{
    ffi::{c_char, CString},
    ops::{Deref, DerefMut},
    path::Path,
};

use crate::{
    error::{error, Error},
    ffi,
};

#[derive(Debug)]
pub struct CompressionFailError(());

impl std::fmt::Display for CompressionFailError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "could not compress data")
    }
}

impl std::error::Error for CompressionFailError {}

/// Owned buffer for holding onto memory returned by [`compress_data`] and [`decompress_data`].
/// Automatically frees the memory when dropped.
#[derive(Debug)]
pub struct DataBuf {
    buffer: *mut u8,
    len: usize,
}
impl Drop for DataBuf {
    fn drop(&mut self) {
        unsafe {
            ffi::MemFree(self.buffer.cast());
        }
    }
}
impl Deref for DataBuf {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        unsafe {
            std::slice::from_raw_parts(self.buffer, self.len)
        }
    }
}
impl DerefMut for DataBuf {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe {
            std::slice::from_raw_parts_mut(self.buffer, self.len)
        }
    }
}
impl AsRef<[u8]> for DataBuf {
    fn as_ref(&self) -> &[u8] {
        self.deref()
    }
}
impl AsMut<[u8]> for DataBuf {
    fn as_mut(&mut self) -> &mut [u8] {
        self.deref_mut()
    }
}

/// Compress data (DEFLATE algorythm)
/// ```rust
/// use raylib::prelude::*;
/// let data = compress_data(b"11111").unwrap();
/// let expected: &[u8] = &[1, 5, 0, 250, 255, 49, 49, 49, 49, 49];
/// assert_eq!(data, expected);
/// ```
pub fn compress_data(data: &[u8]) -> Result<DataBuf, CompressionFailError> {
    let data_len = data.len();
    assert!(data_len <= i32::MAX as usize, "invalid data length");
    let mut out_length: i32 = 0;
    // CompressData doesn't actually modify the data, but the header is wrong
    let buffer = unsafe {
        ffi::CompressData(data.as_ptr() as *mut _, data_len as i32, &mut out_length)
    };

    if buffer.is_null() {
        return Err(CompressionFailError(()));
    }

    assert!(out_length >= 1, "non-null buffer length should never be zero or negative");
    Ok(DataBuf {
        buffer,
        len: out_length as usize,
    })
}

/// Decompress data (DEFLATE algorythm)
/// ```rust
/// use raylib::prelude::*;
/// let input: &[u8] = &[1, 5, 0, 250, 255, 49, 49, 49, 49, 49];
/// let expected: &[u8] = b"11111";
/// let data = decompress_data(input).unwrap();
/// assert_eq!(data, expected);
/// ```
pub fn decompress_data(data: &[u8]) -> Result<DataBuf, CompressionFailError> {
    let data_len = data.len();
    assert!(data_len <= i32::MAX as usize, "invalid data length");
    let mut out_length: i32 = 0;
    // CompressData doesn't actually modify the data, but the header is wrong
    let buffer = unsafe {
        ffi::DecompressData(data.as_ptr() as *mut _, data_len as i32, &mut out_length)
    };

    if buffer.is_null() {
        return Err(CompressionFailError(()));
    }

    assert!(out_length >= 1, "non-null buffer length should never be zero or negative");
    Ok(DataBuf {
        buffer,
        len: out_length as usize,
    })
}

fn path_to_bytes<P: AsRef<Path>>(path: P) -> Vec<u8> {
    #[cfg(unix)] {
        use std::os::unix::ffi::OsStrExt;
        path.as_ref().as_os_str().as_bytes().to_vec()
    }
    #[cfg(not(unix))] {
        path.as_ref().to_string_lossy().to_string().into_bytes()
    }
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

// Decode Base64 data
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
