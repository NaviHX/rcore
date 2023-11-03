use crate::trap::trap_return;

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct TaskContext {
    ra: usize,
    sp: usize,
    s: [usize; 12],
}

// Receive an sp pointing to TrapContext in Kernenl Stack
// Wrap the init trap restore context to task context
pub fn goto_restore(kernel_sp: usize) -> TaskContext {
    extern "C" {
        fn __restore();
    }
    let __restore = __restore as usize;

    TaskContext {
        ra: __restore,
        sp: kernel_sp,
        s: [0; 12],
    }
}

impl TaskContext {
    pub fn zero_init() -> Self {
        Self {
            ra: 0,
            sp: 0,
            s: [0; 12],
        }
    }

    pub fn goto_trap_return(kernel_stack_ptr: usize) -> Self {
        Self {
            ra: trap_return as usize,
            sp: kernel_stack_ptr,
            s: [0; 12],
        }
    }
}
