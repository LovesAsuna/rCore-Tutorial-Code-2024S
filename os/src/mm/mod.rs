//! Memory management implementation
//!
//! SV39 page-based virtual-memory architecture for RV64 systems, and
//! everything about memory management, like frame allocator, page table,
//! map area and memory set, is implemented here.
//!
//! Every task or process has a memory_set to control its virtual memory.

pub use address::{PhysAddr, PhysPageNum, StepByOne, VirtAddr, VirtPageNum};
use address::VPNRange;
pub use frame_allocator::{frame_alloc, frame_dealloc, FrameTracker};
pub use memory_set::{KERNEL_SPACE, kernel_token, MapPermission, MemorySet};
pub use memory_set::remap_test;
pub use page_table::{
    PageTable, PageTableEntry, translated_byte_buffer, translated_refmut,
    translated_str, UserBuffer,
};
use page_table::PTEFlags;

mod address;
mod frame_allocator;
mod heap_allocator;
mod memory_set;
pub(crate) mod page_table;

/// initiate heap allocator, frame allocator and kernel space
pub fn init() {
    heap_allocator::init_heap();
    frame_allocator::init_frame_allocator();
    KERNEL_SPACE.exclusive_access().activate();
}
