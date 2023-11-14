use alloc::sync::Arc;
use lazy_static::lazy_static;

use crate::{trap::context::TrapContext, upsync::UPSyncCell};

use super::{context::TaskContext, manager::fetch_task, TaskControlBlock, TaskStatus, switch::__switch};

pub struct Processor {
    current: Option<Arc<TaskControlBlock>>,
    idle_task_context: TaskContext,
}

impl Processor {
    pub fn new() -> Self {
        Self {
            current: None,
            idle_task_context: TaskContext::zero_init(),
        }
    }

    pub fn get_idle_task_context_ptr(&mut self) -> *mut TaskContext {
        &mut self.idle_task_context as *mut _
    }
}

lazy_static! {
    /// For single processor model
    pub static ref PROCESSOR: UPSyncCell<Processor> = unsafe { UPSyncCell::new(Processor::new()) };
}

impl Processor {
    pub fn take_current(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.current.take()
    }

    pub fn current(&self) -> Option<Arc<TaskControlBlock>> {
        self.current.as_ref().map(|t| t.clone())
    }
}

pub fn take_current_task() -> Option<Arc<TaskControlBlock>> {
    PROCESSOR.exclusive_access().take_current()
}

pub fn current_task() -> Option<Arc<TaskControlBlock>> {
    PROCESSOR.exclusive_access().current()
}

pub fn current_user_token() -> usize {
    let task = current_task().unwrap();
    let x = task.inner_exclusive_access().get_user_token(); x
}

pub fn current_trap_context() -> &'static mut TrapContext {
    current_task()
        .unwrap()
        .inner_exclusive_access()
        .get_trap_context()
}

/// The idle control flow.
pub fn run_tasks() {
    loop {
        let mut processor = PROCESSOR.exclusive_access();
        if let Some(task) = fetch_task() {
            let idle_task_context_ptr = processor.get_idle_task_context_ptr();

            let mut task_inner = task.inner_exclusive_access();
            let next_task_context_ptr = &task_inner.task_context as *const TaskContext;
            task_inner.task_status = TaskStatus::Running;
            drop(task_inner);

            processor.current = Some(task);
            drop(processor);

            unsafe {
                __switch(idle_task_context_ptr, next_task_context_ptr);
            }
        }
    }
}

/// Give processor back to idle control flow, which can determine which task to run next and run
/// it. However, you can run determine the next task and switch to it in this function too.
pub fn schedule(switched_task_context_ptr: *mut TaskContext) {
    let idle_task_context_ptr = PROCESSOR.exclusive_access().get_idle_task_context_ptr();

    unsafe {
        __switch(switched_task_context_ptr, idle_task_context_ptr);
    }
}
