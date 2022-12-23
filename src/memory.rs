pub mod heap_alloc;

pub fn init() {
    heap_alloc::kernel_init_heap_allocator();
}
