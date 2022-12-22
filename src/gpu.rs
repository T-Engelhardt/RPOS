//! GPU code.

use core::ptr;

use embedded_graphics::{
    pixelcolor::{Bgr888, Rgb888},
    prelude::*,
};

#[allow(non_camel_case_types)]
#[derive(Debug)]
pub enum ColorDepth {
    RGB24,
    ARGB32,
    UNKNOWN,
}

impl ColorDepth {
    pub fn determine_depth(input: u32) -> ColorDepth {
        match input {
            24 => Self::RGB24,
            32 => Self::ARGB32,
            _ => Self::UNKNOWN,
        }
    }
    /*
    /// get representation of a pixel
    ///
    /// RGB24   calc_pixel(...) as u32 ignore 4 MSB
    /// ARGB32  calc_pixel(...) as u32
    pub fn calc_pixel(&self, red: usize, blue: usize, green: usize, alpha: Option<usize>) -> usize {
        match &self {
            ColorDepth::RGB24 => ColorDepth::calc_rgb24(red, blue, green, None),
            ColorDepth::ARGB32 => ColorDepth::calc_rgb24(red, blue, green, alpha),
            ColorDepth::UNKNOWN => panic!("UNKNOWN ColorDepth"),
        }
    }

    fn calc_rgb24(red: usize, blue: usize, green: usize, alpha: Option<usize>) -> usize {
        0
    }
    fn calc_argb32(red: usize, blue: usize, green: usize, alpha: Option<usize>) -> usize {
        0
    }
    */
}

pub struct Display {
    pub width: u32,
    pub height: u32,
    pub depth: ColorDepth,          // bits per color
    pub fr_ptr: Option<*const u32>, // framepuffer base
    pub fr_length: usize,           // framepuffer length
}

/// # SAFTEY
///
/// fr_ptr is memory mapped and should thread save
unsafe impl Send for Display {}

impl DrawTarget for Display {
    type Color = Bgr888;

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
            if let Some(ptr) = self.fr_ptr {
                unsafe {
                    ptr::write_volatile(
                        ptr.offset(index.try_into().unwrap()) as *mut u32,
                        byte_array_to_u32(color.to_le_bytes()),
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

fn byte_array_to_u32(src: [u8; 3]) -> u32 {
    ((src[0] as u32) << 16) + ((src[1] as u32) << 8) + ((src[2] as u32) << 0)
}
