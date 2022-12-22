use core::{fmt, mem::size_of, ptr};

use crate::{
    bsp::driver::MAILBOX,
    console, debug, driver,
    gpu::{Display, GPU_FONT},
    info, println, synchronization,
    synchronization::NullLock,
    warn,
};
use embedded_graphics::{mono_font::MonoTextStyle, pixelcolor::Rgb888, prelude::*, text::Text};

struct VideoInner {
    display: Option<Display>,
    cursor_x: u32,
    cursor_y: u32,
    chars_written: usize,
    chars_read: usize,
}

//--------------------------------------------------------------------------------------------------
// Public Definitions
//--------------------------------------------------------------------------------------------------

/// Representation of the VideoCore.
pub struct Video {
    inner: NullLock<VideoInner>,
}

impl VideoInner {
    pub const unsafe fn new() -> Self {
        Self {
            display: None,
            cursor_x: 0,
            cursor_y: 0,
            chars_written: 0,
            chars_read: 0,
        }
    }

    pub fn init(&mut self) {
        self.display = MailBox::request_framebuffer(&MAILBOX);
        if let Some(display) = &self.display {
            info!(
                "Found Display {} x {} depth {:?}",
                display.width, display.height, display.depth
            );
            if let Some(ptr) = display.fp_ptr {
                debug!(
                    "Framebuffer at {:?} with length {} bytes",
                    ptr, display.fp_len
                )
            }
        }
    }

    fn write_char(&mut self, c: char) {
        // hack to create &str
        let mut b = [0; 2];

        self.write_str(c.encode_utf8(&mut b));
    }

    pub fn write_str(&mut self, text: &str) {
        if let Some(display) = &mut self.display {
            // GPU FONT is not thread save
            GPU_FONT.lock(|font| {
                warn!("x: {}, y: {}", self.cursor_x, self.cursor_y);
                warn!(
                    "x: {}, y: {}",
                    self.cursor_x * font.character_size.width,
                    ((self.cursor_y * font.character_size.height) + font.character_size.height)
                );

                // TODO
                // we write per string NOT char
                // maybe needed to split text

                // Create a new character style
                let style = MonoTextStyle::new(font, Rgb888::WHITE);

                // Create a text at position (x, y) and draw it using the previously defined style
                let _ = Text::new(
                    text,
                    Point::new(
                        (self.cursor_x * font.character_size.width) as i32,
                        ((self.cursor_y * font.character_size.height) + font.character_size.height)
                            as i32,
                    ),
                    style,
                )
                .draw(display);
            });
        } else {
            warn!("No Display found");
        }
    }

    // display test image
    pub fn test_image(&mut self) {
        if let Some(display) = &mut self.display {
            if let Some(ptr) = display.fp_ptr {
                unsafe {
                    ptr::write_bytes::<u32>(
                        ptr as *mut u32,
                        u8::MIN,
                        display.fp_len / size_of::<u32>(),
                    );
                }

                // print Text
                let text = "Hello Rust!";

                // GPU FONT is not thread save
                GPU_FONT.lock(|font| {
                    // Create a new character style
                    let style = MonoTextStyle::new(font, Rgb888::RED);

                    // Create a text at position (0, 0) and draw it using the previously defined style
                    let _ = Text::new(
                        text,
                        Point::new(0, font.character_size.height.try_into().unwrap()),
                        style,
                    )
                    .draw(display);
                });
            } else {
                warn!("No framepuffer found");
            }
        } else {
            warn!("No Display found");
        }
    }
}

/// Implementing `core::fmt::Write` enables usage of the `format_args!` macros, which in turn are
/// used to implement the `kernel`'s `print!` and `println!` macros. By implementing `write_str()`,
/// we get `write_fmt()` automatically.
///
/// The function takes an `&mut self`, so it must be implemented for the inner struct.
///
/// See [`src/print.rs`].
///
/// [`src/print.rs`]: ../../print/index.html
impl fmt::Write for VideoInner {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_str(s);

        Ok(())
    }
}

//--------------------------------------------------------------------------------------------------
// Public Code
//--------------------------------------------------------------------------------------------------

impl Video {
    pub const COMPATIBLE: &'static str = "BCM VideoCore IV";

    /// Create an instance.
    pub const unsafe fn new() -> Self {
        Self {
            inner: NullLock::new(VideoInner::new()),
        }
    }

    // TODO REMOVE
    pub fn write_str(&self, text: &str) {
        self.inner.lock(|inner| inner.write_str(text))
    }

    pub fn test_image(&self) {
        self.inner.lock(|inner| inner.test_image())
    }
}

//------------------------------------------------------------------------------
// OS Interface Code
//------------------------------------------------------------------------------
use synchronization::interface::Mutex;

use super::MailBox;

impl driver::interface::DeviceDriver for Video {
    fn compatible(&self) -> &'static str {
        Self::COMPATIBLE
    }

    unsafe fn init(&self) -> Result<(), &'static str> {
        self.inner.lock(|inner| inner.init());

        Ok(())
    }
}

impl console::interface::Write for Video {
    /// Passthrough of `args` to the `core::fmt::Write` implementation, but guarded by a Mutex to
    /// serialize access.
    fn write_char(&self, c: char) {
        self.inner.lock(|inner| inner.write_char(c));
    }

    fn write_fmt(&self, args: core::fmt::Arguments) -> core::fmt::Result {
        // Fully qualified syntax for the call to `core::fmt::Write::write_fmt()` to increase
        // readability.
        self.inner.lock(|inner| fmt::Write::write_fmt(inner, args))
    }

    fn flush(&self) {}
}

impl console::interface::Read for Video {
    fn clear_rx(&self) {}
}

impl console::interface::Statistics for Video {}

impl console::interface::All for Video {}
