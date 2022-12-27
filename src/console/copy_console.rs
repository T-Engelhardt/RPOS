//! Copy console.

use crate::synchronization::{interface::Mutex, NullLock};

use super::interface;
use core::fmt;

//--------------------------------------------------------------------------------------------------
// Private Definitions
//--------------------------------------------------------------------------------------------------

const NUM_CONSOLES: usize = 5;

struct ConsoleInner {
    next_index: usize,
    list: [Option<Console>; NUM_CONSOLES],
}

//--------------------------------------------------------------------------------------------------
// Public Definitions
//--------------------------------------------------------------------------------------------------

#[derive(Copy, Clone)]
pub struct Console {
    console: &'static (dyn interface::All + Sync),
}

pub struct ConsoleManger {
    inner: NullLock<ConsoleInner>,
}

//--------------------------------------------------------------------------------------------------
// Global instances
//--------------------------------------------------------------------------------------------------

pub static CONSOLE_MANGER: ConsoleManger = ConsoleManger::new();

//--------------------------------------------------------------------------------------------------
// Private Code
//--------------------------------------------------------------------------------------------------

impl ConsoleInner {
    /// Create an instance.
    pub const fn new() -> Self {
        Self {
            next_index: 0,
            list: [None; NUM_CONSOLES],
        }
    }
}

//--------------------------------------------------------------------------------------------------
// Public Code
//--------------------------------------------------------------------------------------------------

impl Console {
    pub fn new(console: &'static (dyn interface::All + Sync)) -> Self {
        Self { console }
    }
}

/// Return a reference to the global DriverManager.
pub fn console_manger() -> &'static ConsoleManger {
    &CONSOLE_MANGER
}

impl ConsoleManger {
    /// Create an instance.
    pub const fn new() -> Self {
        Self {
            inner: NullLock::new(ConsoleInner::new()),
        }
    }

    /// Register a console
    pub fn register_console(&self, console: Console) {
        self.inner.lock(|inner| {
            inner.list[inner.next_index] = Some(console);
            inner.next_index += 1;
        })
    }

    /// Helper for iterating over registered consoles.
    fn for_each_console<'a>(&'a self, f: impl FnMut(&'a Console)) {
        self.inner
            .lock(|inner| inner.list.iter().filter_map(|x| x.as_ref()).for_each(f))
    }
}

impl interface::Write for ConsoleManger {
    fn write_char(&self, c: char) {
        self.for_each_console(|console| console.console.write_char(c))
    }

    fn write_fmt(&self, args: fmt::Arguments) -> fmt::Result {
        self.inner.lock::<fmt::Result>(|inner| {
            // iterate only on Some(console)
            // doesn't uses for_each_console since we want to return fmt::Result
            for opt_console in inner.list.iter().filter_map(|x| x.as_ref()) {
                opt_console.console.write_fmt(args)?
            }
            fmt::Result::Ok(())
        })
    }

    fn flush(&self) {
        self.for_each_console(|console| console.console.flush())
    }
}

impl interface::Read for ConsoleManger {
    fn clear_rx(&self) {
        self.for_each_console(|console| console.console.clear_rx())
    }
}

impl interface::Statistics for ConsoleManger {}
impl interface::All for ConsoleManger {}
