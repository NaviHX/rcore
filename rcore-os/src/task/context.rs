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
    extern "C" { fn __restore(); }
    let __restore = __restore as usize;

    TaskContext {
        ra: __restore,
        sp: kernel_sp,
        s: [0; 12],
    }
}
