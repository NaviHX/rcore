use core::arch::{asm, global_asm};

use crate::trap::context::TrapContext;

pub use crate::config::{MAX_APP_NUM, APP_SIZE_LIMIT, APP_BASEADDR};

global_asm!(include_str!("link_app.asm"));

pub fn get_num_app() -> usize {
    extern "C" { fn _num_app(); }
    unsafe { (_num_app as usize as *const usize).read_volatile() }
}

pub fn get_app_data(app_id: usize) -> &'static [u8] {
    extern "C" { fn _num_app(); }
    let num_app_ptr = _num_app as usize as *const usize;
    let num_app = get_num_app();
    let app_start = unsafe {
        core::slice::from_raw_parts(num_app_ptr.add(1), num_app + 1)
    };
    assert!(app_id < num_app);
    let start = app_start[app_id];
    let size = app_start[app_id + 1] - start;
    unsafe {
        core::slice::from_raw_parts(start as *const u8, size)
    }
}

pub use crate::config::{KERNEL_STACK_SIZE, USER_STACK_SIZE};

#[derive(Copy, Clone)]
struct KernelStack {
    mem: [u8; KERNEL_STACK_SIZE],
}

#[derive(Copy, Clone)]
struct UserStack {
    mem: [u8; USER_STACK_SIZE],
}

static KERNEL_STACK: [KernelStack; MAX_APP_NUM] = [KernelStack {
    mem: [0; KERNEL_STACK_SIZE],
}; MAX_APP_NUM];

static USER_STACK: [UserStack; MAX_APP_NUM] = [UserStack {
    mem: [0; USER_STACK_SIZE],
}; MAX_APP_NUM];

macro_rules! stack_impl {
    ($stack:ty, $size:expr) => {
        impl $stack {
            fn get_sp(&self) -> usize {
                self.mem.as_ptr() as usize + $size
            }
        }
    };
}

stack_impl!(KernelStack, KERNEL_STACK_SIZE);
stack_impl!(UserStack, USER_STACK_SIZE);

impl KernelStack {
    fn push_context(&self, ctx: TrapContext) -> &TrapContext {
        let new_sp = (self.get_sp() - core::mem::size_of::<TrapContext>()) as *mut TrapContext;
        unsafe { *new_sp = ctx };
        unsafe { new_sp.as_mut().unwrap() }
    }
}
