//! GPU code.

use core::ptr;

use embedded_graphics::{pixelcolor::Rgb888, prelude::*};

#[allow(non_camel_case_types)]
#[derive(Debug)]
pub enum ColorDepth {
    BGRA32,
    ARGB32,
    UNSUPPORTED,
}

impl ColorDepth {
    /// Determine the ColorDepth
    pub fn determine_depth(input: u32, blue_first: bool) -> ColorDepth {
        match input {
            32 => {
                if blue_first {
                    Self::BGRA32
                } else {
                    Self::ARGB32
                }
            }
            _ => Self::UNSUPPORTED,
        }
    }

    /// Return byte array of Rgb888 into u32 based on the Color Depth
    pub fn byte_array_to_u32(&self, src: [u8; 3]) -> u32 {
        match self {
            ColorDepth::BGRA32 => {
                ((src[0] as u32) << 0) + ((src[1] as u32) << 8) + ((src[2] as u32) << 16)
            }
            ColorDepth::ARGB32 => {
                ((src[0] as u32) << 16) + ((src[1] as u32) << 8) + ((src[2] as u32) << 0)
            }
            ColorDepth::UNSUPPORTED => panic!("UNSUPPORTED Color Depth"),
        }
    }
}

pub struct Display {
    pub width: u32,
    pub height: u32,
    pub depth: ColorDepth,          // bits per color
    pub fp_ptr: Option<*const u32>, // framepuffer base
    pub fp_len: usize,              // framepuffer length
}

/// # SAFTEY
///
/// fp_ptr is memory mapped and should thread save
unsafe impl Send for Display {}

impl DrawTarget for Display {
    type Color = Rgb888;

    // `Display` uses a framebuffer and doesn't need to communicate with the display
    // controller to draw pixel, which means that drawing operations can never fail. To reflect
    // this the type `Infallible` was chosen as the `Error` type.
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        for Pixel(coord, color) in pixels.into_iter() {
            if coord.x < 0 || coord.x > self.width.try_into().unwrap() {
                return Ok(());
            }
            if coord.y < 0 || coord.y > self.height.try_into().unwrap() {
                return Ok(());
            }

            // Calculate the index in the framebuffer.
            let index: u32 = (coord.x + coord.y * self.width as i32).try_into().unwrap();

            //self.framebuffer[index as usize] = color.luma();
            if let Some(ptr) = self.fp_ptr {
                unsafe {
                    ptr::write_volatile(
                        ptr.offset(index.try_into().unwrap()) as *mut u32,
                        self.depth.byte_array_to_u32(color.to_le_bytes()),
                    )
                }
            }
        }

        Ok(())
    }
}

impl OriginDimensions for Display {
    fn size(&self) -> Size {
        Size::new(self.width, self.height)
    }
}
