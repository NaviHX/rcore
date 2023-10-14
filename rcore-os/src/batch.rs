#![allow(unused)]

use core::{
    arch::{asm, global_asm},
    cell::{RefCell, RefMut},
};
use lazy_static::lazy_static;

global_asm!(include_str!("link_app.asm"));

use crate::{log, trap::context::TrapContext, sbi::shutdown};

const MAX_APP_NUM: usize = 10;
pub const APP_BASEADDR: usize = 0x80400000;
const APP_SIZE_LIMIT: usize = 0x20000;

pub struct AppManager {
    num_app: usize,
    current_app: usize,
    app_start: [usize; MAX_APP_NUM + 1],
}

impl AppManager {
    unsafe fn load_app(&self, app_id: usize) {
        if app_id >= self.num_app {
            log!("All apps have run! Shutdown!");
            shutdown();
        }

        log!("Load app_{}", app_id);
        core::slice::from_raw_parts_mut(APP_BASEADDR as *mut u8, APP_SIZE_LIMIT).fill(0);
        let app_src = core::slice::from_raw_parts(
            self.app_start[app_id] as *mut u8,
            self.app_start[app_id + 1] - self.app_start[app_id],
        );
        let app_dst = core::slice::from_raw_parts_mut(APP_BASEADDR as *mut u8, app_src.len());
        app_dst.copy_from_slice(app_src);
        asm!("fence.i"); // Force CPU read the new app memory
    }

    pub fn print_app_info(&self) {
        log!("{} apps to run", self.num_app);
        for i in 0..self.num_app {
            log!(
                "App {}: [{}, {}]",
                i,
                self.app_start[i],
                self.app_start[i + 1]
            );
        }
    }

    pub fn get_current_app(&self) -> usize {
        self.current_app
    }

    pub fn get_app_num(&self) -> usize {
        self.num_app
    }

    pub fn move_to_next_app(&mut self) {
        self.current_app += 1;
    }
}

/// This cell is safe only when being used in single-processor environment
pub struct UPSyncCell<T> {
    inner: RefCell<T>,
}

impl<T> UPSyncCell<T> {
    /// SAFETY: The inner struct is only used in uniprocessor
    pub unsafe fn new(value: T) -> Self {
        Self {
            inner: RefCell::new(value),
        }
    }

    pub fn exclusive_access(&self) -> RefMut<'_, T> {
        self.inner.borrow_mut()
    }
}

unsafe impl<T> Sync for UPSyncCell<T> {}

lazy_static! {
    static ref APP_MANAGER: UPSyncCell<AppManager> = unsafe {
        extern "C" {
            fn _num_app();
        }

        let num_app_ptr = _num_app as usize as *const usize;
        let num_app = num_app_ptr.read_volatile();
        let app_start_raw: &[usize] = core::slice::from_raw_parts(num_app_ptr.add(1), num_app + 1);
        let mut app_start = [0; MAX_APP_NUM + 1];
        app_start[..=num_app].copy_from_slice(app_start_raw);

        UPSyncCell::new(AppManager {
            num_app,
            current_app: 0,
            app_start,
        })
    };
}

const USER_STACK_SIZE: usize = 0x2000;
const KERNEL_STACK_SIZE: usize = 0x2000;

struct KernelStack {
    mem: [u8; KERNEL_STACK_SIZE],
}

struct UserStack {
    mem: [u8; USER_STACK_SIZE],
}

static KERNEL_STACK: KernelStack = KernelStack {
    mem: [0; KERNEL_STACK_SIZE],
};
static USER_STACK: UserStack = UserStack {
    mem: [0; USER_STACK_SIZE],
};

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

pub fn run_next_app() -> ! {
    // Load the next app from the app manager
    let mut app_manager = APP_MANAGER.exclusive_access();
    let current_app = app_manager.get_current_app();
    unsafe {
        app_manager.load_app(current_app);
    }
    app_manager.move_to_next_app();
    drop(app_manager);

    // restore the new app
    extern "C" {
        fn __restore(ctx_addr: usize) -> !;
    }
    let trap_context = TrapContext::app_init_context(APP_BASEADDR, USER_STACK.get_sp());
    let supervisor_sp = KERNEL_STACK.push_context(trap_context) as *const _ as usize;
    unsafe { __restore(supervisor_sp) }
}

pub fn print_app_info() {
    APP_MANAGER.exclusive_access().print_app_info();
}

pub fn get_num_app() -> usize {
    APP_MANAGER.exclusive_access().get_app_num()
}
