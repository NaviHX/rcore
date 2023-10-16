use super::context::TaskContext;
use core::arch::global_asm;

global_asm!(include_str!("switch.asm"));

extern "C" {
    pub fn __switch(
        current_task_context_ptr: *mut TaskContext,
        next_task_context_ptr: *const TaskContext,
    );
}
