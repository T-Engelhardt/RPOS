//! Copy console.

use super::interface;
use core::fmt;

//--------------------------------------------------------------------------------------------------
// Public Definitions
//--------------------------------------------------------------------------------------------------

pub struct CopyConsole {
    console1: Option<&'static (dyn interface::All + Sync)>,
    console2: Option<&'static (dyn interface::All + Sync)>,
}

//--------------------------------------------------------------------------------------------------
// Global instances
//--------------------------------------------------------------------------------------------------

pub static mut COPY_CONSOLE: CopyConsole = CopyConsole {
    console1: None,
    console2: None,
};

//--------------------------------------------------------------------------------------------------
// Public Code
//--------------------------------------------------------------------------------------------------

/// Registers up to 2 console's
///
/// # SAFTEY
/// static mut CONSOLE is only changed at boot
pub fn register_console(new_console: &'static (dyn interface::All + Sync)) {
    unsafe {
        if COPY_CONSOLE.console1.is_none() {
            COPY_CONSOLE.console1 = Some(new_console);
            return;
        }

        if COPY_CONSOLE.console2.is_none() {
            COPY_CONSOLE.console2 = Some(new_console);
            return;
        }
    };
}

impl interface::Write for CopyConsole {
    fn write_char(&self, c: char) {
        if let Some(console) = unsafe { COPY_CONSOLE.console1 } {
            console.write_char(c);
        }
        if let Some(console) = unsafe { COPY_CONSOLE.console2 } {
            console.write_char(c);
        }
    }

    fn write_fmt(&self, args: fmt::Arguments) -> fmt::Result {
        if let Some(console) = unsafe { COPY_CONSOLE.console1 } {
            console.write_fmt(args)?;
        }
        if let Some(console) = unsafe { COPY_CONSOLE.console2 } {
            console.write_fmt(args)?;
        }
        fmt::Result::Ok(())
    }

    fn flush(&self) {
        if let Some(console) = unsafe { COPY_CONSOLE.console1 } {
            console.flush();
        }
        if let Some(console) = unsafe { COPY_CONSOLE.console2 } {
            console.flush();
        }
    }
}

impl interface::Read for CopyConsole {
    fn clear_rx(&self) {
        if let Some(console) = unsafe { COPY_CONSOLE.console1 } {
            console.clear_rx();
        }
        if let Some(console) = unsafe { COPY_CONSOLE.console2 } {
            console.clear_rx();
        }
    }
}

impl interface::Statistics for CopyConsole {}
impl interface::All for CopyConsole {}
