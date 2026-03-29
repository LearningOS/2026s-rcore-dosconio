//!Implementation of [`TaskManager`]
use super::TaskControlBlock;
use crate::sync::UPSafeCell;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use lazy_static::*;
///A array of `TaskControlBlock` that is thread-safe
pub struct TaskManager {
    ready_queue: VecDeque<Arc<TaskControlBlock>>,
}

/// A simple FIFO scheduler.
impl TaskManager {
    ///Creat an empty TaskManager
    pub fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
        }
    }
    /// Add process back to ready queue
	#[allow(warnings)]
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        // stride little->big while front->back
        let _seq = &mut self.ready_queue;
        let _seq_len = _seq.len();
        // sorting insert
        if _seq_len <= 1 {
            _seq.push_back(task);
        } 
        else {
            for i in 2.._seq_len {
                //info!("task {} stride {}", i - 1, _seq[i - 1].get_proc_stride())
                if task.get_proc_stride() > _seq[i].get_proc_stride() {
                    _seq.insert(i - 1, task); // not 0, because 0 is the current running task
                    // info!("task insert {} in {}+1", i - 1, _seq.len() - 1);
                    return;
                }
            }
            _seq.push_back(task);
        }
    }
    /// Take a process out of the ready queue
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.ready_queue.pop_front()
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
