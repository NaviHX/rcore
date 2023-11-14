use crate::{
    config::{PAGE_SIZE, TRAMPOLINE},
    loader::KERNEL_STACK_SIZE,
    mem::{memory_set::MapPermission, KERNEL_SPACE, address::{VirtPageNum, VirtAddr}}, debug,
};

use super::pid::PidHandle;

pub struct KernelStack {
    pid: usize,
}

pub fn kernel_stack_position(app_id: usize) -> (usize, usize) {
    let top = TRAMPOLINE - app_id * (KERNEL_STACK_SIZE + PAGE_SIZE);
    let bot = top - KERNEL_STACK_SIZE;
    (bot, top)
}

impl KernelStack {
    pub fn new(pid_handle: &PidHandle) -> Self {
        let pid: usize = pid_handle.0;
        let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(pid);
        debug!("Mapping kernel stack: {:x}..{:x}", kernel_stack_bottom, kernel_stack_top);
        KERNEL_SPACE.exclusive_access().insert_framed_area(
            kernel_stack_bottom.into(),
            kernel_stack_top.into(),
            MapPermission::R | MapPermission::W,
        );
        KernelStack { pid }
    }

    pub fn push_on_top<T: Sized>(&self, value: T) -> *mut T {
        let kernel_stack_top = self.get_top();
        let ptr_mut = (kernel_stack_top - core::mem::size_of::<T>()) as *mut T;
        unsafe { *ptr_mut = value; }
        ptr_mut
    }

    pub fn get_top(&self) -> usize {
        let (_, kernel_stack_top) = kernel_stack_position(self.pid);
        kernel_stack_top
    }
}

impl Drop for KernelStack {
    fn drop(&mut self) {
        let (bot, top) = kernel_stack_position(self.pid);
        let start_vpn: VirtPageNum = VirtAddr::from(bot).floor();
        debug!("Kernel stack starting from {:x}..{:x} released", bot, top);
        KERNEL_SPACE.exclusive_access().remove_area_with_start_vpn(start_vpn);
    }
}
