use crate::upsync::UPSyncCell;
use alloc::{collections::VecDeque, sync::Arc};
use lazy_static::lazy_static;

use super::TaskControlBlock;

pub trait TaskManager {
    fn new() -> Self;
    fn add(&mut self, task: Arc<TaskControlBlock>);
    fn fetch(&mut self) -> Option<Arc<TaskControlBlock>>;
}

pub struct FIFOTaskManager {
    ready_queue: VecDeque<Arc<TaskControlBlock>>,
}

impl TaskManager for FIFOTaskManager {
    fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
        }
    }

    fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push_back(task);
    }

    fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.ready_queue.pop_front()
    }
}

type ImplTaskManager = FIFOTaskManager;
lazy_static! {
    pub static ref TASK_MANAGER: UPSyncCell<ImplTaskManager> =
        unsafe { UPSyncCell::new(FIFOTaskManager::new()) };
}

pub fn add_task(task: Arc<TaskControlBlock>) {
    TASK_MANAGER.exclusive_access().add(task);
}

pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    TASK_MANAGER.exclusive_access().fetch()
}
