use super::address::*;
use alloc::vec::{self, Vec};

pub trait FrameAllocator {
    fn alloc(&mut self) -> Option<PhysPageNum>;
    fn dealloc(&mut self, ppn: PhysPageNum);
    fn create_with(start: PhysPageNum, end: PhysPageNum) -> Self;
}

pub struct StackFrameAllocator {
    current: usize,
    end: usize,
    recycled: Vec<usize>,
}

impl FrameAllocator for StackFrameAllocator {
    fn alloc(&mut self) -> Option<PhysPageNum> {
        if let Some(ppn) = self.recycled.pop() {
            Some(ppn.into())
        } else if self.current == self.end {
            None
        } else {
            self.current += 1;
            Some((self.current - 1).into())
        }
    }

    fn dealloc(&mut self, ppn: PhysPageNum) {
        let ppn = ppn.0;
        if ppn >= self.current || self.recycled.iter().any(|v| *v == ppn) {
            panic!("Frame ppn = {:#x} has not been allocated!", ppn);
        }
        self.recycled.push(ppn);
    }

    fn create_with(start: PhysPageNum, end: PhysPageNum) -> Self {
        let mut allocator = Self::new();
        allocator.init(start, end);
        allocator
    }
}

impl StackFrameAllocator {
    pub fn init(&mut self, l: PhysPageNum, r: PhysPageNum) {
        self.current = l.0;
        self.end = r.0;
    }

    pub fn new() -> Self {
        Self {
            current: 0,
            end: 0,
            recycled: vec![],
        }
    }
}

use crate::config::MEMORY_END;
use crate::upsync::UPSyncCell;
use lazy_static::lazy_static;
type FrameAllocatorImpl = StackFrameAllocator;
lazy_static! {
    pub static ref FRAME_ALLOCATOR: UPSyncCell<FrameAllocatorImpl> =
        unsafe { UPSyncCell::new(FrameAllocatorImpl::new()) };
}

pub fn init_frame_allocator() {
    extern "C" {
        fn kernel_end();
    }

    FRAME_ALLOCATOR.exclusive_access().init(
        PhysAddr::from(kernel_end as usize).ceil(),
        PhysAddr::from(MEMORY_END).floor(),
    );
}

pub struct FrameTracker {
    pub ppn: PhysPageNum,
}

impl FrameTracker {
    pub fn new(ppn: PhysPageNum) -> Self {
        let bytes_array = ppn.get_byte_array();
        for i in bytes_array {
            *i = 0;
        }
        Self { ppn }
    }
}

impl Drop for FrameTracker {
    fn drop(&mut self) {
        frame_dealloc(self.ppn);
    }
}

pub fn frame_alloc() -> Option<FrameTracker> {
    FRAME_ALLOCATOR
        .exclusive_access()
        .alloc()
        .map(FrameTracker::new)
}

pub fn frame_dealloc(ppn: PhysPageNum) {
    FRAME_ALLOCATOR.exclusive_access().dealloc(ppn);
}
