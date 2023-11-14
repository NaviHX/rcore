use crate::{upsync::UPSyncCell, debug};
use alloc::vec::Vec;
use lazy_static::lazy_static;

pub const IDLE_PID: usize = 0;

pub struct PidHandle(pub usize);

impl Drop for PidHandle {
    fn drop(&mut self) {
        debug!("Pid {} recycled", self.0);
        PID_ALLOCATOR.exclusive_access().dealloc(self.0)
    }
}

impl From<PidHandle> for usize {
    fn from(val: PidHandle) -> Self {
        val.0
    }
}

struct PidAllocator {
    current: usize,
    recycled: Vec<usize>,
}

impl PidAllocator {
    pub fn new() -> PidAllocator {
        PidAllocator {
            current: 0,
            recycled: vec![],
        }
    }

    pub fn alloc(&mut self) -> PidHandle {
        if let Some(pid) = self.recycled.pop() {
            PidHandle(pid)
        } else {
            self.current += 1;
            PidHandle(self.current - 1)
        }
    }

    pub fn dealloc(&mut self, pid: usize) {
        self.recycled.push(pid);
    }
}

lazy_static! {
    static ref PID_ALLOCATOR: UPSyncCell<PidAllocator> =
        unsafe { UPSyncCell::new(PidAllocator::new()) };
}

pub fn pid_alloc() -> PidHandle {
    PID_ALLOCATOR.exclusive_access().alloc()
}
