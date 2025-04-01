#[macro_use]
mod macros;

pub mod audio;
pub mod automation;
pub mod callbacks;
pub mod camera;
pub mod collision;
pub mod color;
pub mod data;
pub mod drawing;
pub mod error;
pub mod file;

pub mod input;
pub mod logging;
pub mod math;
pub mod misc;
pub mod models;
pub mod shaders;
pub mod text;
pub mod texture;
pub mod vr;
pub mod window;

use raylib_sys::TraceLogLevel;

use crate::ffi;
use std::ffi::CString;
use std::marker::PhantomData;

// shamelessly stolen from imgui
#[macro_export]
macro_rules! rstr {
    ($e:tt) => ({
        #[allow(unused_unsafe)]
        unsafe {
          std::ffi::CStr::from_bytes_with_nul_unchecked(concat!($e, "\0").as_bytes())
        }
    });
    ($e:tt, $($arg:tt)*) => ({
        #[allow(unused_unsafe)]
        unsafe {
          std::ffi::CString::new(format!($e, $($arg)*)).unwrap()
        }
    })
}

/// This token is used to ensure certain functions are only running on the same
/// thread raylib was initialized from. This is useful for architectures like macos
/// where cocoa can only be called from one thread.
#[derive(Clone, Debug)]
pub struct RaylibThread(PhantomData<*const ()>);

/// The main interface into the Raylib API.
///
/// This is the way in which you will use the vast majority of Raylib's functionality. A `RaylibHandle` can be constructed using the [`init_window`] function or through a [`RaylibBuilder`] obtained with the [`init`] function.
///
/// [`init_window`]: fn.init_window.html
/// [`RaylibBuilder`]: struct.RaylibBuilder.html
/// [`init`]: fn.init.html
#[derive(Debug)]
pub struct RaylibHandle(()); // inner field is private, preventing manual construction

impl Drop for RaylibHandle {
    fn drop(&mut self) {
        unsafe {
            if ffi::IsWindowReady() {
                ffi::CloseWindow();
                // NOTE(IOI_XD): If imgui is enabled, we don't call the destructor here because we're using a context that Rust expects to free, and the only other thing in that function is the free'ing of FontTexture...an action which causes a segfault.
                // It then gets successfully replaced if rlImGuiReloadFonts is called, so we'll take it.
            }
        }
    }
}

/// A builder that allows more customization of the game window shown to the user before the `RaylibHandle` is created.
#[derive(Debug, Default)]
pub struct RaylibBuilder {
    fullscreen_mode: bool,
    window_resizable: bool,
    window_undecorated: bool,
    window_transparent: bool,
    msaa_4x_hint: bool,
    vsync_hint: bool,
    log_level: TraceLogLevel,
    width: i32,
    height: i32,
    title: String,
    #[cfg(feature = "imgui")]
    imgui_theme: crate::imgui::ImGuiTheme,
}

/// Creates a `RaylibBuilder` for choosing window options before initialization.
pub fn init(width: i32, height: i32, title: &str) -> RaylibBuilder {
    RaylibBuilder {
        width,
        height,
        title: title.to_string(),
        ..Default::default()
    }
}

impl RaylibBuilder {
    /// Sets the window to be fullscreen.
    pub fn fullscreen(&mut self) -> &mut Self {
        self.fullscreen_mode = true;
        self
    }

    /// Set the builder's log level.
    pub fn log_level(&mut self, level: TraceLogLevel) -> &mut Self {
        self.log_level = level;
        self
    }
    /// Sets the window to be resizable.
    pub fn resizable(&mut self) -> &mut Self {
        self.window_resizable = true;
        self
    }

    /// Sets the window to be undecorated (without a border).
    pub fn undecorated(&mut self) -> &mut Self {
        self.window_undecorated = true;
        self
    }

    /// Sets the window to be transparent.
    pub fn transparent(&mut self) -> &mut Self {
        self.window_transparent = true;
        self
    }

    /// Hints that 4x MSAA (anti-aliasing) should be enabled. The system's graphics drivers may override this setting.
    pub fn msaa_4x(&mut self) -> &mut Self {
        self.msaa_4x_hint = true;
        self
    }

    /// Hints that vertical sync (VSync) should be enabled. The system's graphics drivers may override this setting.
    pub fn vsync(&mut self) -> &mut Self {
        self.vsync_hint = true;
        self
    }

    #[cfg(feature = "imgui")]
    /// Set the theme to be used for imgui.
    pub fn imgui_theme(&mut self, theme: crate::imgui::ImGuiTheme) -> &mut Self {
        self.imgui_theme = theme;
        self
    }

    /// Builds and initializes a Raylib window.
    ///
    /// # Panics
    ///
    /// Attempting to initialize Raylib more than once will result in a panic.
    pub fn build(&self) -> Result<(RaylibHandle, RaylibThread), InitWindowError> {
        use crate::consts::ConfigFlags::*;
        let mut flags = 0u32;
        if self.fullscreen_mode {
            flags |= FLAG_FULLSCREEN_MODE as u32;
        }
        if self.window_resizable {
            flags |= FLAG_WINDOW_RESIZABLE as u32;
        }
        if self.window_undecorated {
            flags |= FLAG_WINDOW_UNDECORATED as u32;
        }
        if self.window_transparent {
            flags |= FLAG_WINDOW_TRANSPARENT as u32;
        }
        if self.msaa_4x_hint {
            flags |= FLAG_MSAA_4X_HINT as u32;
        }
        if self.vsync_hint {
            flags |= FLAG_VSYNC_HINT as u32;
        }

        unsafe {
            ffi::SetConfigFlags(flags as u32);
        }

        unsafe {
            ffi::SetTraceLogLevel(self.log_level as i32);
        }

        let rl = init_window(self.width, self.height, &self.title)?;

        #[cfg(feature = "imgui")]
        unsafe {
            crate::imgui::init_imgui_context(self.imgui_theme == crate::imgui::ImGuiTheme::Dark);
        }

        Ok((rl, RaylibThread(PhantomData)))
    }
}

#[derive(Debug)]
pub enum InitWindowError {
    DoubleInit,
    CreateFailed,
}

impl std::fmt::Display for InitWindowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InitWindowError::DoubleInit => write!(f, "Attempted to initialize raylib-rs more than once!"),
            InitWindowError::CreateFailed => write!(f, "Attempting to create window failed!"),
        }
    }
}

impl std::error::Error for InitWindowError {}

/// Initializes window and OpenGL context.
///
/// # Panics
///
/// Attempting to initialize Raylib more than once will result in a panic.
fn init_window(width: i32, height: i32, title: &str) -> Result<RaylibHandle, InitWindowError> {
    if unsafe { ffi::IsWindowReady() } {
        Err(InitWindowError::DoubleInit)
    } else {
        unsafe {
            let c_title = CString::new(title).unwrap();
            ffi::InitWindow(width, height, c_title.as_ptr());
        }
        if !unsafe { ffi::IsWindowReady() } {
            return Err(InitWindowError::CreateFailed);
        }

        Ok(RaylibHandle(()))
    }
}
