pub mod context;
pub mod manager;
pub mod pid;
pub mod processor;
pub mod stack;
pub mod switch;

use core::cell::RefMut;

use crate::{
    config::{PAGE_SIZE, TRAMPOLINE, TRAP_CONTEXT},
    debug,
    loader::{self, get_app_data, get_app_data_by_name, KERNEL_STACK_SIZE, MAX_APP_NUM},
    log,
    mem::{
        address::{PhysPageNum, VirtAddr},
        memory_set::{MapPermission, MemorySet},
        KERNEL_SPACE,
    },
    sbi::shutdown,
    trap::{context::TrapContext, trap_handler},
    upsync::UPSyncCell,
};
use alloc::{sync::Arc, sync::Weak, vec::Vec};
use context::TaskContext;
use lazy_static::lazy_static;

use self::{
    manager::add_task,
    pid::{pid_alloc, PidHandle},
    processor::{schedule, take_current_task},
    stack::KernelStack,
    switch::__switch,
};

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum TaskStatus {
    UnInit,
    Ready,
    Running,
    Exited,
    Zombie,
}

pub struct TaskControlBlock {
    pub pid: PidHandle,
    pub kernel_stack: KernelStack,

    inner: UPSyncCell<InnerTaskControlBlock>,
}

pub struct InnerTaskControlBlock {
    pub trap_context_ppn: PhysPageNum,
    pub base_size: usize,
    pub task_context: TaskContext,
    pub task_status: TaskStatus,
    pub memory_set: MemorySet,
    pub parent: Option<Weak<TaskControlBlock>>,
    pub children: Vec<Arc<TaskControlBlock>>,
    pub exit_code: i32,
}

impl InnerTaskControlBlock {
    pub fn get_trap_context(&self) -> &'static mut TrapContext {
        self.trap_context_ppn.get_mut()
    }

    pub fn get_user_token(&self) -> usize {
        self.memory_set.token()
    }

    pub fn get_status(&self) -> TaskStatus {
        self.task_status
    }

    pub fn is_zombie(&self) -> bool {
        self.get_status() == TaskStatus::Zombie
    }
}

impl TaskControlBlock {
    pub fn inner_exclusive_access(&self) -> RefMut<'_, InnerTaskControlBlock> {
        self.inner.exclusive_access()
    }

    pub fn new(elf_data: &[u8]) -> Self {
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);
        let trap_context_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();
        let pid = pid_alloc();
        let kernel_stack = KernelStack::new(&pid);
        let kernel_stack_top = kernel_stack.get_top();

        let tcb = Self {
            pid,
            kernel_stack,
            inner: unsafe {
                UPSyncCell::new(InnerTaskControlBlock {
                    trap_context_ppn,
                    base_size: user_sp,
                    task_context: TaskContext::goto_trap_return(kernel_stack_top),
                    task_status: TaskStatus::Ready,
                    memory_set,
                    parent: None,
                    children: vec![],
                    exit_code: 0,
                })
            },
        };

        let trap_context = tcb.inner_exclusive_access().get_trap_context();
        *trap_context = TrapContext::app_init_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.exclusive_access().token(),
            kernel_stack_top,
            trap_handler as usize,
        );

        tcb
    }

    pub fn get_pid(&self) -> usize {
        self.pid.0
    }

    /// CAUTIONS: After calling this function, user space pointers and trap context pointer may be invalid.
    pub fn exec(&self, elf_data: &[u8]) {
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);
        let trap_context_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();
        let mut inner = self.inner_exclusive_access();

        inner.memory_set = memory_set;
        inner.trap_context_ppn = trap_context_ppn;
        let trap_context = inner.get_trap_context();
        *trap_context = TrapContext::app_init_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.exclusive_access().token(),
            self.kernel_stack.get_top(),
            trap_handler as usize,
        );
    }

    pub fn fork(self: &Arc<TaskControlBlock>) -> Arc<TaskControlBlock> {
        let mut parent_inner = self.inner_exclusive_access();
        let memory_set = MemorySet::from_existed_user_space(&parent_inner.memory_set);
        let trap_context_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();
        let pid_handle = pid_alloc();
        let kernel_stack = KernelStack::new(&pid_handle);
        let kernel_stack_top = kernel_stack.get_top();
        let tcb = Arc::new(TaskControlBlock {
            pid: pid_handle,
            kernel_stack,
            inner: unsafe {
                UPSyncCell::new(InnerTaskControlBlock {
                    trap_context_ppn,
                    base_size: parent_inner.base_size,
                    task_context: TaskContext::goto_trap_return(kernel_stack_top),
                    task_status: TaskStatus::Ready,
                    memory_set,
                    parent: Some(Arc::downgrade(self)),
                    children: vec![],
                    exit_code: 0,
                })
            },
        });

        parent_inner.children.push(tcb.clone());
        let trap_context = tcb.inner_exclusive_access().get_trap_context();
        trap_context.kernel_sp = kernel_stack_top;

        tcb
    }
}

lazy_static! {
    pub static ref INIT_PROC: Arc<TaskControlBlock> = Arc::new(TaskControlBlock::new(
        get_app_data_by_name("initproc").unwrap()
    ));
}

pub fn add_init_proc() {
    add_task(INIT_PROC.clone());
}

// pub struct TaskManager {
//     num_app: usize,
//     inner: UPSyncCell<InnerTaskManager>,
// }
//
// struct InnerTaskManager {
//     tasks: Vec<TaskControlBlock>,
//     current_task: usize,
// }
//
// impl TaskManager {
//     pub fn run_first_task(&self) {
//         let mut inner = self.inner.exclusive_access();
//         inner.current_task = 0;
//         let task0 = &mut inner.tasks[0];
//         task0.task_status = TaskStatus::Running;
//         let next_task_context_ptr = &task0.task_context as *const _;
//
//         drop(inner);
//
//         let mut cur = TaskContext::zero_init();
//         let cur = &mut cur as *mut _;
//         unsafe { __switch(cur, next_task_context_ptr) };
//         panic!("Unable to run the first task!");
//     }
//
//     fn mark_current_suspended(&self) {
//         let mut inner = self.inner.exclusive_access();
//         let current_task_id = inner.current_task;
//         inner.tasks[current_task_id].task_status = TaskStatus::Ready;
//     }
//
//     fn mark_current_exited(&self) {
//         let mut inner = self.inner.exclusive_access();
//         let current_task_id = inner.current_task;
//         inner.tasks[current_task_id].task_status = TaskStatus::Exited;
//     }
//
//     /// Schedule logic
//     /// Now we just schedule the first ready task after current task
//     fn find_next_task(&self) -> Option<usize> {
//         let inner = self.inner.exclusive_access();
//         let current = inner.current_task;
//         let len = self.num_app;
//         (current + 1..=current + len) // include the current app itself
//             .map(|id| id % len)
//             .find(|&id| inner.tasks[id].task_status == TaskStatus::Ready)
//     }
//
//     pub fn run_next_task(&self) {
//         if let Some(next_task_id) = self.find_next_task() {
//             let mut inner = self.inner.exclusive_access();
//             let cur_id = inner.current_task;
//             inner.current_task = next_task_id;
//             let next_task = &mut inner.tasks[next_task_id];
//             next_task.task_status = TaskStatus::Running;
//             let next_task_context_ptr = &next_task.task_context as *const _;
//             let cur = &mut inner.tasks[cur_id].task_context as *mut _;
//             drop(inner);
//
//             debug!("Select task {}", next_task_id);
//             unsafe { __switch(cur, next_task_context_ptr) };
//         } else {
//             log!("All tasks completed!");
//             shutdown();
//         }
//     }
// }
//
// lazy_static! {
//     static ref TASK_MANAGER: TaskManager = {
//         let num_app = loader::get_num_app();
//         let mut tasks: Vec<TaskControlBlock> = vec![];
//         for i in 0..num_app {
//             // Init tasks and set Ready
//             tasks.push(TaskControlBlock::new(get_app_data(i), i));
//         }
//
//         TaskManager {
//             num_app,
//             inner: unsafe {
//                 UPSyncCell::new(InnerTaskManager {
//                     tasks,
//                     current_task: 0,
//                 })
//             }
//         }
//     };
// }
//
// pub fn run_first_task() {
//     debug!("Running first task");
//     TASK_MANAGER.run_first_task();
// }

pub fn suspend_and_run_next() {
    let task = take_current_task().unwrap();

    let mut inner_task = task.inner_exclusive_access();
    let cur_task_context_ptr = &mut inner_task.task_context as *mut TaskContext;
    inner_task.task_status = TaskStatus::Ready;
    drop(inner_task);

    add_task(task);
    schedule(cur_task_context_ptr);
}

pub fn exit_and_run_next(exit_code: i32) -> ! {
    debug!("Exit and run next task");

    let task = take_current_task().unwrap();
    let pid = task.get_pid();

    if pid == pid::IDLE_PID {
        log!("Idle Process exit with {}", pid);
        shutdown();
    }

    let mut inner = task.inner_exclusive_access();
    inner.task_status = TaskStatus::Zombie;
    inner.exit_code = exit_code;

    // Move exited process's children to init proc
    {
        let mut init_proc_inner = INIT_PROC.inner_exclusive_access();
        for child in inner.children.iter() {
            child.inner_exclusive_access().parent = Some(Arc::downgrade(&INIT_PROC));
            init_proc_inner.children.push(child.clone());
        }
    }

    inner.children.clear();
    inner.memory_set.recycle_data_pages();
    drop(inner);
    drop(task);

    let mut unused = TaskContext::zero_init();
    schedule(&mut unused as *mut _);

    panic!("Run exited task again");
}

// /// Return Both the bottom and top of the kernel stack of app_id
// pub fn kernel_stack_position(app_id: usize) -> (usize, usize) {
//     let top = TRAMPOLINE - app_id * (KERNEL_STACK_SIZE + PAGE_SIZE);
//     let btm = top - KERNEL_STACK_SIZE;
//     (btm, top)
// }
// impl TaskManager {
//     fn get_current_token(&self) -> usize {
//         let inner = self.inner.borrow();
//         let current = inner.current_task;
//         inner.tasks[current].get_user_token()
//     }
//
//     fn get_current_trap_context(&self) -> &mut TrapContext {
//         let inner = self.inner.borrow();
//         let current = inner.current_task;
//         inner.tasks[current].get_trap_context()
//     }
// }
//
// pub fn get_current_token() -> usize {
//     TASK_MANAGER.get_current_token()
// }
// pub fn get_current_trap_context() -> &'static mut TrapContext {
//     TASK_MANAGER.get_current_trap_context()
// }
