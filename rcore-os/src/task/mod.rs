pub mod context;
pub mod switch;

use crate::{
    loader::{self, MAX_APP_NUM},
    log,
    sbi::shutdown,
    upsync::UPSyncCell,
};
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

#[derive(Copy, Clone)]
pub struct TaskControlBlock {
    pub task_status: TaskStatus,
    pub task_context: TaskContext,
}

pub struct TaskManager {
    num_app: usize,
    inner: UPSyncCell<InnerTaskManager>,
}

struct InnerTaskManager {
    tasks: [TaskControlBlock; MAX_APP_NUM],
    current_app: usize,
}

impl TaskManager {
    pub fn run_first_task(&self) {
        let mut inner = self.inner.exclusive_access();
        inner.current_app = 0;
        let task0 = &mut inner.tasks[0];
        task0.task_status = TaskStatus::Running;
        let next_task_context_ptr = &task0.task_context as *const _;

        drop(inner);

        let mut cur = TaskContext::default();
        let cur = &mut cur as *mut _;
        unsafe { __switch(cur, next_task_context_ptr) };
        panic!("Unable to run the first task!");
    }

    fn mark_current_suspended(&self) {
        let mut inner = self.inner.exclusive_access();
        let current_task_id = inner.current_app;
        inner.tasks[current_task_id].task_status = TaskStatus::Ready;
    }

    fn mark_current_exited(&self) {
        let mut inner = self.inner.exclusive_access();
        let current_task_id = inner.current_app;
        inner.tasks[current_task_id].task_status = TaskStatus::Exited;
    }

    /// Schedule logic
    /// Now we just schedule the first ready task after current task
    fn find_next_task(&self) -> Option<usize> {
        let inner = self.inner.exclusive_access();
        let current = inner.current_app;
        let len = self.num_app;
        (current + 1..=current + len) // include the current app itself
            .map(|id| id % len)
            .find(|&id| inner.tasks[id].task_status == TaskStatus::Ready)
    }

    pub fn run_next_task(&self) {
        if let Some(next_task_id) = self.find_next_task() {
            let mut inner = self.inner.exclusive_access();
            let cur_id = inner.current_app;
            inner.current_app = next_task_id;
            let next_task = &mut inner.tasks[next_task_id];
            next_task.task_status = TaskStatus::Running;
            let next_task_context_ptr = &next_task.task_context as *const _;
            let cur = &mut inner.tasks[cur_id].task_context as *mut _;
            drop(inner);

            unsafe { __switch(cur, next_task_context_ptr) };
        } else {
            log!("All tasks completed!");
            shutdown();
        }
    }
}

lazy_static! {
    static ref TASK_MANAGER: TaskManager = {
        let num_apps = loader::get_num_apps();
        let mut tasks = [TaskControlBlock {
            task_status: TaskStatus::UnInit,
            task_context: TaskContext::default(),
        }; MAX_APP_NUM];
        for i in 0..num_apps {
            // Init tasks and set Ready
            tasks[i].task_context = context::goto_restore(loader::init_app_context(i));
            tasks[i].task_status = TaskStatus::Ready;
        }

        TaskManager {
            num_app: num_apps,
            inner: unsafe {
                UPSyncCell::new(InnerTaskManager {
                    tasks,
                    current_app: 0,
                })
            }
        }
    };
}

pub fn run_first_task() {
    TASK_MANAGER.run_first_task();
}

pub fn suspend_and_run_next() {
    TASK_MANAGER.mark_current_suspended();
    TASK_MANAGER.run_next_task();
}

pub fn exit_and_run_next() -> ! {
    TASK_MANAGER.mark_current_exited();
    TASK_MANAGER.run_next_task();
    panic!("Run exited task again");
}
