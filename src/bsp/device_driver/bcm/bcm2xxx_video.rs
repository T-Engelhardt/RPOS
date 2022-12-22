use core::{fmt, mem::size_of, ptr};

use crate::{
    bsp::driver::MAILBOX,
    console, debug, driver,
    gpu::{Display, GPU_FONT},
    info, synchronization,
    synchronization::NullLock,
    warn,
};
use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::Rgb888,
    prelude::*,
    primitives::{PrimitiveStyleBuilder, Rectangle},
    text::Text,
};

struct VideoInner {
    display: Option<Display>,
    cursor_x: u32,
    cursor_y: u32,
    chars_written: usize,
    chars_read: usize,
    font_width: u32,
    font_height: u32,
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
            font_width: 0,
            font_height: 0,
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

    /// reset/init cursor for console for output
    pub fn reset_console(&mut self) {
        GPU_FONT.lock(|font| {
            self.font_width = font.character_size.width;
            self.font_height = font.character_size.height;
        });
        info!("VideoConsole font {}x{}", self.font_width, self.font_height);
        // cursor needs to be shiftet to accommodate a char
        // of by one !!!
        // Text is rendered by embedded-graphics at the bottom left pixel of the char
        // thats why font heigth - 1 because the the pixel is ..=x and not ..x => open interval
        self.cursor_y = self.font_height - 1;
        self.cursor_x = 0;

        // set framebuffer to zero
        if let Some(display) = &mut self.display {
            if let Some(ptr) = display.fp_ptr {
                unsafe {
                    ptr::write_bytes::<u32>(
                        ptr as *mut u32,
                        u8::MIN,
                        display.fp_len / size_of::<u32>(),
                    );
                }
            }
        }
    }

    fn write_char(&mut self, c: char) {
        // hack to create &str
        let mut b = [0; 2];

        self.write_str(c.encode_utf8(&mut b));
    }

    fn write_str(&mut self, text: &str) {
        if let Some(display) = &mut self.display {
            let mut cutoff: u32 = text.len() as u32;
            let mut next_row: bool = false;

            if self.cursor_x + text.len() as u32 * self.font_width > display.width {
                let overflow_char = (self.cursor_x / self.font_width + text.len() as u32)
                    - display.width / self.font_width;
                cutoff -= overflow_char;
                next_row = true;
            }

            // Draw text
            // GPU FONT is not thread save
            GPU_FONT.lock(|font| {
                // Create a new character style
                let style = MonoTextStyle::new(font, Rgb888::WHITE);

                // DEBUG
                // cant use fmt:: here since fmt calls this fn
                //warn!("!x: {} y: {}", self.cursor_x, self.cursor_y);

                // Create a text at position (x, y) and draw it using the previously defined style
                let _ = Text::new(
                    text.get(0..cutoff as usize).unwrap(),
                    Point::new(self.cursor_x as i32, self.cursor_y as i32),
                    style,
                )
                .draw(display);
            });

            // calc the cursor
            if !next_row {
                self.cursor_x += text.len() as u32 * self.font_width;
            } else {
                self.cursor_x = 0;
                self.cursor_y += self.font_height;
            }

            // TODO check for new line in text
            // check if new line at the end
            if text.ends_with('\n') {
                self.cursor_x = 0;
                self.cursor_y += self.font_height;
            }

            // run out of screen at the bottom
            if self.cursor_y >= display.height {
                self.cursor_y -= self.font_height;
                self.scroll_video_console();
            }

            // print rest string
            if next_row {
                self.write_str(text.get(cutoff as usize..text.len()).unwrap())
            }
        }
    }

    fn scroll_video_console(&self) {
        // TODO
        if let Some(display) = &self.display {
            if let Some(fp) = display.fp_ptr {
                // move every char row up except the first
                for row in self.font_height as isize..display.height as isize {
                    unsafe {
                        ptr::copy_nonoverlapping::<u32>(
                            fp.offset(row * display.width as isize),
                            fp.offset((row - self.font_height as isize) * display.width as isize)
                                as *mut u32,
                            display.width as usize,
                        )
                    }
                }
                // empty last row
                for row in display.height - self.font_height..display.height {
                    unsafe {
                        ptr::write_bytes(
                            fp.offset((row * display.width) as isize) as *mut u32,
                            u8::MIN,
                            display.width as usize,
                        )
                    }
                }
            }
        };
    }

    // display test image
    pub fn _test_image(&mut self) {
        if let Some(display) = &mut self.display {
            if let Some(ptr) = display.fp_ptr {
                unsafe {
                    ptr::write_bytes::<u32>(
                        ptr as *mut u32,
                        u8::MIN,
                        display.fp_len / size_of::<u32>(),
                    );
                }

                let style = PrimitiveStyleBuilder::new()
                    .stroke_color(Rgb888::RED)
                    .stroke_width(1)
                    .fill_color(Rgb888::GREEN)
                    .build();

                let _ = Rectangle::new(Point::new(0, 0), Size::new(10, 10))
                    .into_styled(style)
                    .draw(display);

                let _ = Rectangle::new(Point::new(0, 10), Size::new(10, 10))
                    .into_styled(style)
                    .draw(display);

                let style = MonoTextStyle::new(&FONT_6X10, Rgb888::WHITE);

                // Create a text at position (20, 30) and draw it using the previously defined style
                let _ = Text::new("Hello Rust!", Point::new(10, 9), style).draw(display);

                let _ = Text::new("Hello Rust!", Point::new(10, 19), style).draw(display);
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

    pub fn reset_console(&self) {
        self.inner.lock(|inner| inner.reset_console())
    }

    // DEBUG
    /*
    pub fn write_str(&self, text: &str) {
        self.inner.lock(|inner| inner.write_str(text))
    }
    */

    pub fn _test_image(&self) {
        self.inner.lock(|inner| inner._test_image())
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

impl console::interface::Statistics for Video {
    fn chars_written(&self) -> usize {
        self.inner.lock(|inner| inner.chars_written)
    }

    fn chars_read(&self) -> usize {
        self.inner.lock(|inner| inner.chars_read)
    }
}

impl console::interface::All for Video {}
