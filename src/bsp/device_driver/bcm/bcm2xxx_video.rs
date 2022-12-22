use core::{mem::size_of, ptr};

use crate::{
    bsp::driver::MAILBOX, debug, driver, gpu::Display, info, synchronization,
    synchronization::NullLock, warn,
};

use embedded_graphics::{
    mono_font::{ascii::FONT_10X20, MonoTextStyle},
    pixelcolor::{Bgr888, Rgb888},
    prelude::*,
    text::Text,
};

struct VideoInner {
    display: Option<Display>,
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
        Self { display: None }
    }

    pub fn init(&mut self) {
        self.display = MailBox::request_framebuffer(&MAILBOX);
        if let Some(display) = &self.display {
            info!(
                "Found Display {} x {} depth {:?}",
                display.width, display.height, display.depth
            );
            if let Some(ptr) = display.fr_ptr {
                debug!(
                    "Framebuffer at {:?} with length {} bytes",
                    ptr, display.fr_length
                )
            }
        }
    }

    // display white image
    pub fn test_image(&mut self) {
        if let Some(display) = &mut self.display {
            if let Some(ptr) = display.fr_ptr {
                unsafe {
                    ptr::write_bytes::<u32>(
                        ptr as *mut u32,
                        u8::MIN,
                        display.fr_length / size_of::<u32>(),
                    );
                }

                // print Text
                let text = "Hello Rust!";
                let font = FONT_10X20;

                // Create a new character style
                let style = MonoTextStyle::new(&font, Bgr888::BLUE);

                // Create a text at position (0, 0) and draw it using the previously defined style
                let _ = Text::new(
                    text,
                    Point::new(0, font.character_size.height.try_into().unwrap()),
                    style,
                )
                .draw(display);
            } else {
                warn!("No framepuffer found");
            }
        } else {
            warn!("No Display found");
        }
    }
}

impl Video {
    pub const COMPATIBLE: &'static str = "BCM VideoCore IV";

    /// Create an instance.
    pub const unsafe fn new() -> Self {
        Self {
            inner: NullLock::new(VideoInner::new()),
        }
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
