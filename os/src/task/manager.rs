//!Implementation of [`TaskManager`]
use alloc::collections::BinaryHeap;
use alloc::sync::Arc;
use core::cmp::Reverse;

use lazy_static::*;

use crate::sync::UPSafeCell;
use crate::task::task::ComparableTCB;

use super::TaskControlBlock;

///A array of `TaskControlBlock` that is thread-safe
pub struct TaskManager {
    ready_queue: BinaryHeap<Reverse<ComparableTCB>>,
}

const BIG_STRIDE: usize = 0xFFFF - 1;
/// A simple FIFO scheduler.
impl TaskManager {
    ///Creat an empty TaskManager
    pub fn new() -> Self {
        Self {
            ready_queue: BinaryHeap::new(),
        }
    }
    /// Add process back to ready queue
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push(Reverse(ComparableTCB(task)));
    }
    /// Take a process out of the ready queue
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.ready_queue.pop().map(|task| {
            let task = task.0.0;
            let mut tcb = task.inner_exclusive_access();
            let pass = BIG_STRIDE / tcb.priority;
            tcb.stride += pass;
            drop(tcb);
            task
        })
    }
}

lazy_static! {
    /// TASK_MANAGER instance through lazy_static!
    pub static ref TASK_MANAGER: UPSafeCell<TaskManager> =
        unsafe { UPSafeCell::new(TaskManager::new()) };
}

/// Add process to ready queue
pub fn add_task(task: Arc<TaskControlBlock>) {
    //trace!("kernel: TaskManager::add_task");
    TASK_MANAGER.exclusive_access().add(task);
}

/// Take a process out of the ready queue
pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    //trace!("kernel: TaskManager::fetch_task");
    TASK_MANAGER.exclusive_access().fetch()
}
