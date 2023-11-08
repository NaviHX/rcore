pub mod heap_allocator;
pub mod frame_allocator;
pub mod address;
pub mod page_table;
pub mod memory_set;

pub use memory_set::KERNEL_SPACE;

use crate::log;

pub fn init() {
    heap_allocator::init_heap();
    log!("Heap allocator inited");
    frame_allocator::init_frame_allocator();
    log!("Frame allocator inited");
    KERNEL_SPACE.exclusive_access().activate();
    log!("Kernel memory set inited");
}
