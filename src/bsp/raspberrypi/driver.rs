// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Copyright (c) 2018-2022 Andre Richter <andre.o.richter@gmail.com>

//! BSP driver support.

use super::memory::map::mmio;
use crate::{
    bsp::device_driver,
    console::{self, COPY_CONSOLE},
    driver as generic_driver,
};
use core::sync::atomic::{AtomicBool, Ordering};

//--------------------------------------------------------------------------------------------------
// Global instances
//--------------------------------------------------------------------------------------------------

static PL011_UART: device_driver::PL011Uart =
    unsafe { device_driver::PL011Uart::new(mmio::PL011_UART_START) };
static GPIO: device_driver::GPIO = unsafe { device_driver::GPIO::new(mmio::GPIO_START) };
pub static MAILBOX: device_driver::MailBox =
    unsafe { device_driver::MailBox::new(mmio::MAIL_START) };
pub static VIDEOCORE: device_driver::Video = unsafe { device_driver::Video::new() };

//--------------------------------------------------------------------------------------------------
// Private Code
//--------------------------------------------------------------------------------------------------

/// This must be called only after successful init of the UART driver.
fn post_init_uart() -> Result<(), &'static str> {
    //console::register_console(&PL011_UART);
    console::copy_console::register_console(&PL011_UART);

    Ok(())
}

/// This must be called only after successful init of the GPIO driver.
fn post_init_gpio() -> Result<(), &'static str> {
    GPIO.map_pl011_uart();
    Ok(())
}

/// This must be called only after successful init of the Mailbox driver.
fn post_init_mailbox() -> Result<(), &'static str> {
    Ok(())
}

/// This must be called only after successful init of the Video driver.
fn post_init_video() -> Result<(), &'static str> {
    //console::register_console(&VIDEOCORE);
    //console::copy_console::register_console(&VIDEOCORE);

    Ok(())
}

fn driver_uart() -> Result<(), &'static str> {
    let uart_descriptor =
        generic_driver::DeviceDriverDescriptor::new(&PL011_UART, Some(post_init_uart));
    generic_driver::driver_manager().register_driver(uart_descriptor);

    Ok(())
}

fn driver_gpio() -> Result<(), &'static str> {
    let gpio_descriptor = generic_driver::DeviceDriverDescriptor::new(&GPIO, Some(post_init_gpio));
    generic_driver::driver_manager().register_driver(gpio_descriptor);

    Ok(())
}

fn driver_mailbox() -> Result<(), &'static str> {
    let mailbox_descriptor =
        generic_driver::DeviceDriverDescriptor::new(&MAILBOX, Some(post_init_mailbox));
    generic_driver::driver_manager().register_driver(mailbox_descriptor);

    Ok(())
}

fn driver_video() -> Result<(), &'static str> {
    let video_descriptor =
        generic_driver::DeviceDriverDescriptor::new(&VIDEOCORE, Some(post_init_video));
    generic_driver::driver_manager().register_driver(video_descriptor);

    Ok(())
}

//--------------------------------------------------------------------------------------------------
// Public Code
//--------------------------------------------------------------------------------------------------

/// Initialize the driver subsystem.
///
/// # Safety
///
/// See child function calls.
///
/// # Note
///
/// Using atomics here relieves us from needing to use `unsafe` for the static variable.
///
/// On `AArch64`, which is the only implemented architecture at the time of writing this,
/// [`AtomicBool::load`] and [`AtomicBool::store`] are lowered to ordinary load and store
/// instructions. They are therefore safe to use even with MMU + caching deactivated.
///
/// [`AtomicBool::load`]: core::sync::atomic::AtomicBool::load
/// [`AtomicBool::store`]: core::sync::atomic::AtomicBool::store
pub unsafe fn init() -> Result<(), &'static str> {
    static INIT_DONE: AtomicBool = AtomicBool::new(false);
    if INIT_DONE.load(Ordering::Relaxed) {
        return Err("Init already done");
    }

    // register COPY Console
    console::register_console(&COPY_CONSOLE);

    driver_uart()?;
    driver_gpio()?;
    driver_mailbox()?;
    driver_video()?;

    INIT_DONE.store(true, Ordering::Relaxed);
    Ok(())
}
