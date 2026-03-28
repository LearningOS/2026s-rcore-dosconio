//! Task management implementation
//!
//! Everything about task management, like starting and switching tasks is
//! implemented here.
//!
//! A single global instance of [`TaskManager`] called `TASK_MANAGER` controls
//! all the tasks in the operating system.
//!
//! Be careful when you see `__switch` ASM function in `switch.S`. Control flow around this function
//! might not be what you expect.

#![allow(warnings)]//{TEMP}

mod context;
mod switch;
#[allow(clippy::module_inception)]
mod task;

use crate::config::MAX_SYSCALL_NUM;
use crate::loader::{get_app_data, get_num_app};
use crate::sync::UPSafeCell;
use crate::trap::TrapContext;
use alloc::vec::Vec;
use lazy_static::*;
use switch::__switch;
pub use task::{TaskControlBlock, TaskStatus};

pub use context::TaskContext;

use crate::mm::{
    MapPermission, VirtAddr
};

/// The task manager, where all the tasks are managed.
///
/// Functions implemented on `TaskManager` deals with all task state transitions
/// and task context switching. For convenience, you can find wrappers around it
/// in the module level.
///
/// Most of `TaskManager` are hidden behind the field `inner`, to defer
/// borrowing checks to runtime. You can see examples on how to use `inner` in
/// existing functions on `TaskManager`.
pub struct TaskManager {
    /// total number of tasks
    num_app: usize,
    /// use inner value to get mutable access
    inner: UPSafeCell<TaskManagerInner>,
}

/// The task manager inner in 'UPSafeCell'
struct TaskManagerInner {
    /// task list
    tasks: Vec<TaskControlBlock>,
    /// id of current `Running` task
    current_task: usize,
}

lazy_static! {
    /// a `TaskManager` global instance through lazy_static!
    pub static ref TASK_MANAGER: TaskManager = {
        println!("init TASK_MANAGER");
        let num_app = get_num_app();
        println!("num_app = {}", num_app);
        let mut tasks: Vec<TaskControlBlock> = Vec::new();
        for i in 0..num_app {
            tasks.push(TaskControlBlock::new(get_app_data(i), i));
        }
        TaskManager {
            num_app,
            inner: unsafe {
                UPSafeCell::new(TaskManagerInner {
                    tasks,
                    current_task: 0,
                })
            },
        }
    };
}

impl TaskManager {
    /// Run the first task in task list.
    ///
    /// Generally, the first task in task list is an idle task (we call it zero process later).
    /// But in ch4, we load apps statically, so the first task is a real app.
    fn run_first_task(&self) -> ! {
        let mut inner = self.inner.exclusive_access();
        let next_task = &mut inner.tasks[0];
        next_task.task_status = TaskStatus::Running;
        let next_task_cx_ptr = &next_task.task_cx as *const TaskContext;
        drop(inner);
        let mut _unused = TaskContext::zero_init();
        // before this, we should drop local variables that must be dropped manually
        unsafe {
            __switch(&mut _unused as *mut _, next_task_cx_ptr);
        }
        panic!("unreachable in run_first_task!");
    }

    /// Change the status of current `Running` task into `Ready`.
    fn mark_current_suspended(&self) {
        let mut inner = self.inner.exclusive_access();
        let cur = inner.current_task;
        inner.tasks[cur].task_status = TaskStatus::Ready;
    }

    /// Change the status of current `Running` task into `Exited`.
    fn mark_current_exited(&self) {
        let mut inner = self.inner.exclusive_access();
        let cur = inner.current_task;
        inner.tasks[cur].task_status = TaskStatus::Exited;
    }

    /// Find next task to run and return task id.
    ///
    /// In this case, we only return the first `Ready` task in task list.
    fn find_next_task(&self) -> Option<usize> {
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        (current + 1..current + self.num_app + 1)
            .map(|id| id % self.num_app)
            .find(|id| inner.tasks[*id].task_status == TaskStatus::Ready)
    }

    /// Get the current 'Running' task's token.
    fn get_current_token(&self) -> usize {
        let inner = self.inner.exclusive_access();
        inner.tasks[inner.current_task].get_user_token()
    }

    /// Get the current 'Running' task's trap contexts.
    fn get_current_trap_cx(&self) -> &'static mut TrapContext {
        let inner = self.inner.exclusive_access();
        inner.tasks[inner.current_task].get_trap_cx()
    }

    /// Change the current 'Running' task's program break
    pub fn change_current_program_brk(&self, size: i32) -> Option<usize> {
        let mut inner = self.inner.exclusive_access();
        let cur = inner.current_task;
        inner.tasks[cur].change_program_brk(size)
    }

    /// Switch current `Running` task to the task we have found,
    /// or there is no `Ready` task and we can exit with all applications completed
    fn run_next_task(&self) {
        if let Some(next) = self.find_next_task() {
            let mut inner = self.inner.exclusive_access();
            let current = inner.current_task;
            inner.tasks[next].task_status = TaskStatus::Running;
            inner.current_task = next;
            let current_task_cx_ptr = &mut inner.tasks[current].task_cx as *mut TaskContext;
            let next_task_cx_ptr = &inner.tasks[next].task_cx as *const TaskContext;
            drop(inner);
            // before this, we should drop local variables that must be dropped manually
            unsafe {
                __switch(current_task_cx_ptr, next_task_cx_ptr);
            }
            // go back to user mode
        } else {
            panic!("All applications completed!");
        }
    }

    /// Get start time
    fn get_start_time(&self) -> isize {
        let inner = self.inner.exclusive_access();
        let res = inner.tasks[inner.current_task].start_time;
        drop(inner);
        res
    }

    /// Increase syscall times
    fn increase_syscall_times(&self, callno: usize) {
        if callno >= MAX_SYSCALL_NUM {
            panic!("Invalid syscall number!");
        }
        let mut inner = self.inner.exclusive_access();
        let crt = inner.current_task;
        inner.tasks[crt].syscall_times[callno] += 1;
        drop(inner);
    }

    /// Get syscall times
    fn get_syscall_times(&self, systab: &mut [u32; MAX_SYSCALL_NUM]) {
        let inner = self.inner.exclusive_access();
        let crt = inner.current_task;
        for i in 0..MAX_SYSCALL_NUM {
            systab[i] = inner.tasks[crt].syscall_times[i];
        }
        drop(inner);
    }

    ///
    fn get_syscall_times_of(&self, id: usize) -> u32 {
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        inner.tasks[current].syscall_times[id]
    }

	/// mmap
	fn pro_mmap(&self, start: usize, len: usize, port: &MapPermission) -> bool {
		let mut inner = self.inner.exclusive_access();
		let crt = inner.current_task;
		let crt_memset = &mut inner.tasks[crt].memory_set;
		info!("mmap {:#x} to {:#x}", start >> 12, (start + len) >> 12);
		if crt_memset.if_overlap(VirtAddr::from(start), VirtAddr::from(start + len)) {
			drop(inner);
			return false;
		}
		
		crt_memset.insert_framed_area(
			VirtAddr::from(start),
			VirtAddr::from(start + len),
			*port);
		drop(inner);
		true
	}
	
	/// munmap
	fn pro_munmap(&self, start: usize, len: usize) -> bool {
		let mut inner = self.inner.exclusive_access();
		let crt = inner.current_task;
		let crt_memset = &mut inner.tasks[crt].memory_set;
		// info!("!!!!!!!!!!!!!{:#x}!!{:#x}", start, start + len);
		if !crt_memset.if_matched(start, start + len) {
			drop(inner);
			return false;
		}
		crt_memset.remove_area(VirtAddr::from(VirtAddr::from(start).floor()) ,
			VirtAddr::from(VirtAddr::from(start + len).ceil()));
		drop(inner);
		true
	}
}

/// Run the first task in task list.
pub fn run_first_task() {
    TASK_MANAGER.run_first_task();
}

/// Switch current `Running` task to the task we have found,
/// or there is no `Ready` task and we can exit with all applications completed
fn run_next_task() {
    TASK_MANAGER.run_next_task();
}

/// Change the status of current `Running` task into `Ready`.
fn mark_current_suspended() {
    TASK_MANAGER.mark_current_suspended();
}

/// Change the status of current `Running` task into `Exited`.
fn mark_current_exited() {
    TASK_MANAGER.mark_current_exited();
}

/// Suspend the current 'Running' task and run the next task in task list.
pub fn suspend_current_and_run_next() {
    mark_current_suspended();
    run_next_task();
}

/// Exit the current 'Running' task and run the next task in task list.
pub fn exit_current_and_run_next() {
    mark_current_exited();
    run_next_task();
}

/// Get start time (unit/ms)
pub fn get_start_time() -> isize {
    TASK_MANAGER.get_start_time()
}

/// Increase syscall times
pub fn increase_syscall_times(callno: usize) {
    TASK_MANAGER.increase_syscall_times(callno);
}

/// Get syscall times
pub fn get_syscall_times(systab: &mut [u32; MAX_SYSCALL_NUM]) {
    TASK_MANAGER.get_syscall_times(systab);
}

///
pub fn get_syscall_times_of(id: usize) -> u32 {
    TASK_MANAGER.get_syscall_times_of(id)
}

/// Get the current 'Running' task's token.
pub fn current_user_token() -> usize {
    TASK_MANAGER.get_current_token()
}

/// Get the current 'Running' task's trap contexts.
pub fn current_trap_cx() -> &'static mut TrapContext {
    TASK_MANAGER.get_current_trap_cx()
}

/// Change the current 'Running' task's program break
pub fn change_program_brk(size: i32) -> Option<usize> {
    TASK_MANAGER.change_current_program_brk(size)
}

/// ...
pub fn do_task_mmap(start: usize, len: usize, port: usize) -> bool {
    trace!("Paging Map: {:#x}(inc) ~ {:#x}(exc)", start, start + len);
    let mut permission = MapPermission::from_bits((port as u8) << 1).unwrap();
    permission.set(MapPermission::U, true);
    TASK_MANAGER.pro_mmap(start, len, &permission)
} 

/// ...
pub fn do_task_munmap(start: usize, len: usize) -> bool {
    TASK_MANAGER.pro_munmap(start, len)
}
