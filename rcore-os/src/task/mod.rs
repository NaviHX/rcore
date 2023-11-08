pub mod context;
pub mod switch;

use crate::{
    config::{PAGE_SIZE, TRAMPOLINE, TRAP_CONTEXT},
    loader::{self, get_app_data, KERNEL_STACK_SIZE, MAX_APP_NUM},
    log,
    mem::{
        address::{PhysPageNum, VirtAddr},
        memory_set::{MapPermission, MemorySet},
        KERNEL_SPACE,
    },
    sbi::shutdown,
    trap::{context::TrapContext, trap_handler},
    upsync::UPSyncCell, debug,
};
use alloc::vec::Vec;
use context::TaskContext;
use lazy_static::lazy_static;

use self::switch::__switch;

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum TaskStatus {
    UnInit,
    Ready,
    Running,
    Exited,
}

pub struct TaskControlBlock {
    pub task_status: TaskStatus,
    pub task_context: TaskContext,

    pub memory_set: MemorySet,
    pub trap_context_ppn: PhysPageNum,
    pub base_size: usize,
}

pub struct TaskManager {
    num_app: usize,
    inner: UPSyncCell<InnerTaskManager>,
}

struct InnerTaskManager {
    tasks: Vec<TaskControlBlock>,
    current_task: usize,
}

impl TaskManager {
    pub fn run_first_task(&self) {
        let mut inner = self.inner.exclusive_access();
        inner.current_task = 0;
        let task0 = &mut inner.tasks[0];
        task0.task_status = TaskStatus::Running;
        let next_task_context_ptr = &task0.task_context as *const _;

        drop(inner);

        let mut cur = TaskContext::zero_init();
        let cur = &mut cur as *mut _;
        unsafe { __switch(cur, next_task_context_ptr) };
        panic!("Unable to run the first task!");
    }

    fn mark_current_suspended(&self) {
        let mut inner = self.inner.exclusive_access();
        let current_task_id = inner.current_task;
        inner.tasks[current_task_id].task_status = TaskStatus::Ready;
    }

    fn mark_current_exited(&self) {
        let mut inner = self.inner.exclusive_access();
        let current_task_id = inner.current_task;
        inner.tasks[current_task_id].task_status = TaskStatus::Exited;
    }

    /// Schedule logic
    /// Now we just schedule the first ready task after current task
    fn find_next_task(&self) -> Option<usize> {
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        let len = self.num_app;
        (current + 1..=current + len) // include the current app itself
            .map(|id| id % len)
            .find(|&id| inner.tasks[id].task_status == TaskStatus::Ready)
    }

    pub fn run_next_task(&self) {
        if let Some(next_task_id) = self.find_next_task() {
            let mut inner = self.inner.exclusive_access();
            let cur_id = inner.current_task;
            inner.current_task = next_task_id;
            let next_task = &mut inner.tasks[next_task_id];
            next_task.task_status = TaskStatus::Running;
            let next_task_context_ptr = &next_task.task_context as *const _;
            let cur = &mut inner.tasks[cur_id].task_context as *mut _;
            drop(inner);

            debug!("Select task {}", next_task_id);
            unsafe { __switch(cur, next_task_context_ptr) };
        } else {
            log!("All tasks completed!");
            shutdown();
        }
    }
}

lazy_static! {
    static ref TASK_MANAGER: TaskManager = {
        let num_app = loader::get_num_app();
        let mut tasks: Vec<TaskControlBlock> = vec![];
        for i in 0..num_app {
            // Init tasks and set Ready
            tasks.push(TaskControlBlock::new(get_app_data(i), i));
        }

        TaskManager {
            num_app,
            inner: unsafe {
                UPSyncCell::new(InnerTaskManager {
                    tasks,
                    current_task: 0,
                })
            }
        }
    };
}

pub fn run_first_task() {
    debug!("Running first task");
    TASK_MANAGER.run_first_task();
}

pub fn suspend_and_run_next() {
    debug!("Suspend and run next task");
    TASK_MANAGER.mark_current_suspended();
    TASK_MANAGER.run_next_task();
}

pub fn exit_and_run_next() -> ! {
    debug!("Exit and run next task");
    TASK_MANAGER.mark_current_exited();
    TASK_MANAGER.run_next_task();
    panic!("Run exited task again");
}

/// Return Both the bottom and top of the kernel stack of app_id
pub fn kernel_stack_position(app_id: usize) -> (usize, usize) {
    let top = TRAMPOLINE - app_id * (KERNEL_STACK_SIZE + PAGE_SIZE);
    let btm = top - KERNEL_STACK_SIZE;
    (btm, top)
}

impl TaskControlBlock {
    pub fn get_trap_context(&self) -> &'static mut TrapContext {
        self.trap_context_ppn.get_mut()
    }
    pub fn new(elf_data: &[u8], app_id: usize) -> Self {
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);
        let trap_context_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();
        let task_status = TaskStatus::Ready;
        let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(app_id);
        KERNEL_SPACE.exclusive_access().insert_framed_area(
            VirtAddr(kernel_stack_bottom),
            VirtAddr(kernel_stack_top),
            MapPermission::R | MapPermission::W,
        );

        let task_control_block = Self {
            task_status,
            task_context: TaskContext::goto_trap_return(kernel_stack_top),
            memory_set,
            trap_context_ppn,
            base_size: user_sp,
        };

        let trap_context = task_control_block.get_trap_context();
        *trap_context = TrapContext::app_init_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.exclusive_access().token(),
            kernel_stack_top,
            trap_handler as usize,
        );

        task_control_block
    }

    pub fn get_user_token(&self) -> usize {
        self.memory_set.token()
    }
}

impl TaskManager {
    fn get_current_token(&self) -> usize {
        let inner = self.inner.borrow();
        let current = inner.current_task;
        inner.tasks[current].get_user_token()
    }

    fn get_current_trap_context(&self) -> &mut TrapContext {
        let inner = self.inner.borrow();
        let current = inner.current_task;
        inner.tasks[current].get_trap_context()
    }
}

pub fn get_current_token() -> usize {
    TASK_MANAGER.get_current_token()
}
pub fn get_current_trap_context() -> &'static mut TrapContext {
    TASK_MANAGER.get_current_trap_context()
}
