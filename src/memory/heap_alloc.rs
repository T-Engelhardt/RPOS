use crate::{
    bsp, common, debug, info,
    synchronization::{self, NullLock},
};

use core::{
    alloc::{GlobalAlloc, Layout},
    sync::atomic::{AtomicBool, Ordering},
};
use linked_list_allocator::Heap as LinkedListHeap;

//--------------------------------------------------------------------------------------------------
// Public Definitions
//--------------------------------------------------------------------------------------------------

/// A heap allocator that can be lazyily initialized.
pub struct HeapAllocator {
    inner: NullLock<LinkedListHeap>,
}

//--------------------------------------------------------------------------------------------------
// Global instances
//--------------------------------------------------------------------------------------------------

// TODO remove pub
#[global_allocator]
pub static KERNEL_HEAP_ALLOCATOR: HeapAllocator = HeapAllocator::new();

//--------------------------------------------------------------------------------------------------
// Private Code
//--------------------------------------------------------------------------------------------------

#[inline(always)]
fn debug_print_alloc_dealloc(operation: &'static str, ptr: *mut u8, layout: Layout) {
    let size = layout.size();
    let (size_h, size_unit) = common::size_human_readable_ceil(size);

    debug!(
        "Kernel Heap: {}\n      \
        Size:     {:#x} ({} {})\n      \
        Start:    {:?}\n      \
        End excl: {:?}",
        operation,
        size,
        size_h,
        size_unit,
        ptr,
        unsafe { ptr.add(size) },
    );
}

//--------------------------------------------------------------------------------------------------
// Public Code
//--------------------------------------------------------------------------------------------------
use synchronization::interface::Mutex;

#[alloc_error_handler]
fn alloc_error_handler(layout: Layout) -> ! {
    panic!("Allocation error: {:?}", layout)
}

/// Return a reference to the kernel's heap allocator.
pub fn kernel_heap_allocator() -> &'static HeapAllocator {
    &KERNEL_HEAP_ALLOCATOR
}

impl HeapAllocator {
    /// Create an instance.
    pub const fn new() -> Self {
        Self {
            inner: NullLock::new(LinkedListHeap::empty()),
        }
    }

    /// Print the current heap usage.
    pub fn print_usage(&self) {
        let (used, free) = KERNEL_HEAP_ALLOCATOR
            .inner
            .lock(|inner| (inner.used(), inner.free()));

        if used >= 1024 {
            let (used_h, used_unit) = common::size_human_readable_ceil(used);
            info!("      Used: {} Byte ({} {})", used, used_h, used_unit);
        } else {
            info!("      Used: {} Byte", used);
        }

        if free >= 1024 {
            let (free_h, free_unit) = common::size_human_readable_ceil(free);
            info!("      Free: {} Byte ({} {})", free, free_h, free_unit);
        } else {
            info!("      Free: {} Byte", free);
        }
    }
}

unsafe impl GlobalAlloc for HeapAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let result = KERNEL_HEAP_ALLOCATOR
            .inner
            .lock(|inner| inner.allocate_first_fit(layout).ok());

        match result {
            None => core::ptr::null_mut(),
            Some(allocation) => {
                let ptr = allocation.as_ptr();

                debug_print_alloc_dealloc("Allocation", ptr, layout);

                ptr
            }
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        KERNEL_HEAP_ALLOCATOR
            .inner
            .lock(|inner| inner.deallocate(core::ptr::NonNull::new_unchecked(ptr), layout));

        debug_print_alloc_dealloc("Free", ptr, layout);
    }
}

pub fn kernel_init_heap_allocator() {
    static INIT_DONE: AtomicBool = AtomicBool::new(false);
    if INIT_DONE.load(Ordering::Relaxed) {
        return;
    }

    KERNEL_HEAP_ALLOCATOR.inner.lock(|inner| unsafe {
        inner.init(bsp::memory::virt_heap_start(), bsp::memory::heap_size())
    });

    INIT_DONE.store(true, Ordering::Relaxed);
}
