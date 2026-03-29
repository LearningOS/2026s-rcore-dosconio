//! Process management syscalls
//!
#![allow(warnings)]
use alloc::sync::Arc;
use core::mem::size_of;
use crate::{
    config::MAX_SYSCALL_NUM,
    fs::{open_file, OpenFlags},
    mm::{translated_refmut, translated_str, translated_byte_buffer},
    task::{
        add_task, current_task, current_user_token, exit_current_and_run_next,
        suspend_current_and_run_next, TaskStatus,
		do_task_mmap,
        do_task_munmap,
        get_start_time,
        get_syscall_times,
    },
	timer::{get_time_ms, get_time_us},
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// Task information
#[allow(dead_code)]
pub struct TaskInfo {
    /// Task status in it's life cycle
    status: TaskStatus,
    /// The numbers of syscall called by task
    syscall_times: [u32; MAX_SYSCALL_NUM],
    /// Total running time of task
    time: usize,
}

pub fn sys_exit(exit_code: i32) -> ! {
    trace!("kernel:pid[{}] sys_exit", current_task().unwrap().pid.0);
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

pub fn sys_yield() -> isize {
    //trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

pub fn sys_getpid() -> isize {
    trace!("kernel: sys_getpid pid:{}", current_task().unwrap().pid.0);
    current_task().unwrap().pid.0 as isize
}

pub fn sys_fork() -> isize {
    trace!("kernel:pid[{}] sys_fork", current_task().unwrap().pid.0);
    let current_task = current_task().unwrap();
    let new_task = current_task.fork();
    let new_pid = new_task.pid.0;
    // modify trap context of new_task, because it returns immediately after switching
    let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
    // we do not have to move to next instruction since we have done it before
    // for child process, fork returns 0
    trap_cx.x[10] = 0;
    // add new task to scheduler
    add_task(new_task);
    new_pid as isize
}

pub fn sys_exec(path: *const u8) -> isize {
    trace!("kernel:pid[{}] sys_exec", current_task().unwrap().pid.0);
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(app_inode) = open_file(path.as_str(), OpenFlags::RDONLY) {
        let all_data = app_inode.read_all();
        let task = current_task().unwrap();
        task.exec(all_data.as_slice());
        0
    } else {
        -1
    }
}

/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    //trace!("kernel: sys_waitpid");
    let task = current_task().unwrap();
    // find a child process

    // ---- access current PCB exclusively
    let mut inner = task.inner_exclusive_access();
    if !inner
        .children
        .iter()
        .any(|p| pid == -1 || pid as usize == p.getpid())
    {
        return -1;
        // ---- release current PCB
    }
    let pair = inner.children.iter().enumerate().find(|(_, p)| {
        // ++++ temporarily access child PCB exclusively
        p.inner_exclusive_access().is_zombie() && (pid == -1 || pid as usize == p.getpid())
        // ++++ release child PCB
    });
    if let Some((idx, _)) = pair {
        let child = inner.children.remove(idx);
        // confirm that child will be deallocated after being removed from children list
        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.getpid();
        // ++++ temporarily access child PCB exclusively
        let exit_code = child.inner_exclusive_access().exit_code;
        // ++++ release child PCB
        *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;
        found_pid as isize
    } else {
        -2
    }
    // ---- release current PCB automatically
}


/// get time with second and microsecond
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    let time_val = TimeVal {
        sec: us / 1_000_000,
        usec: us % 1_000_000,
    };
    let buffers = translated_byte_buffer(current_user_token(), ts as *const u8, size_of::<TimeVal>());
    let mut time_val_ptr = &time_val as *const _ as *const u8;
    for buffer in buffers {
        unsafe {
            time_val_ptr.copy_to(buffer.as_mut_ptr(), buffer.len());
            time_val_ptr = time_val_ptr.add(buffer.len());
        }
    }
    0
}

/// @dosconio 
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    trace!("kernel: sys_mmap");

    if _port & 0x7 == 0 {
        return -1;// Useless Mapping Page
    }
    if _start & 0xfff != 0 || _port & !(0b111 as usize) != 0{
        return -1;// Tutorial Request
    }
    if _len == 0 {
        return 0;//{ISSUE}
    }

    if do_task_mmap(_start, _len, _port) { 0 } else { -1 }
}

/// @dosconio
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel: sys_munmap");
    if _len == 0 {
        return 0;//{ISSUE}
    }
    //info!(">>>>>>>>>{:#x}!!{:#x}", _start, _start + _len);// any info/error! here will stuck system
    if do_task_munmap(_start, _len) { 0 } else { -1 }
}
///{TODO} change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel:pid[{}] sys_sbrk", current_task().unwrap().pid.0);
    if let Some(old_brk) = current_task().unwrap().change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}

/// @dosconio, HINT: fork + exec =/= spawn
pub fn sys_spawn(_path: *const u8) -> isize {

    let task = current_task().unwrap();
    
    let token = current_user_token();
    let path = translated_str(token, _path);

    if let Some(inode) = open_file(&path, OpenFlags::RDONLY) { //{unchk}
        let v = inode.read_all();
        let new_task = task.fexe(v.as_slice());
        let new_pid = new_task.pid.0;
        add_task(new_task);
        info!("kernel:pid[{}] sys_spawn pid[{}]", task.pid.0, new_pid);
        new_pid as isize
    } else { -1 }
}

/// @dosconio: Set task priority.
pub fn sys_set_priority(_prio: isize) -> isize {
    trace!( "kernel:pid[{}] sys_set_priority", current_task().unwrap().pid.0);
    if _prio < 2 {return -1;}
    let crt = current_task().unwrap();
    let mut res = crt.inner_exclusive_access();
    res.task_priority = _prio as usize;
    _prio
}