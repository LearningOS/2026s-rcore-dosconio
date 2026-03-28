//! Process management syscalls
#![allow(warnings)]

use core::mem::size_of;
use crate::{
    config::MAX_SYSCALL_NUM,
    task::{
        change_program_brk, exit_current_and_run_next, suspend_current_and_run_next, TaskStatus,
    },

	mm::*, task::{
        current_user_token, get_syscall_times_of, do_task_mmap, do_task_munmap
    }, timer::{/*get_time,*/ get_time_us}
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

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// 
pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    let time_val = TimeVal {
        sec: us / 1_000_000,
        usec: us % 1_000_000,
    };
    let buffers = translated_byte_buffer(current_user_token(), _ts as *const u8, size_of::<TimeVal>());
    let mut time_val_ptr = &time_val as *const _ as *const u8;
    for buffer in buffers {
        unsafe {
            time_val_ptr.copy_to(buffer.as_mut_ptr(), buffer.len());
            time_val_ptr = time_val_ptr.add(buffer.len());
        }
    }
    0
}

pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    let token = current_user_token();
    let page_table = PageTable::from_token(token);
    let vpn = VirtAddr::from(id).floor();
    
    match trace_request {
        0 => { // Read
            let pte = page_table.translate(vpn);
            if pte.is_none() || !pte.unwrap().readable() || !pte.unwrap().flags().contains(PTEFlags::U) {
                return -1;
            }
            let buffers = translated_byte_buffer(token, id as *const u8, 1);
            if buffers.is_empty() { -1 } else { buffers[0][0] as isize }
        }
        1 => { // Write
            let pte = page_table.translate(vpn);
            if pte.is_none() || !pte.unwrap().writable() || !pte.unwrap().flags().contains(PTEFlags::U) {
                return -1;
            }
            let mut buffers = translated_byte_buffer(token, id as *mut u8, 1);
            if buffers.is_empty() { return -1; }
            unsafe { buffers[0][0] = (data & 0xFF) as u8; }
            0
        }
        2 => { // Query ... 
            if id >= MAX_SYSCALL_NUM {
                return -1;
            }
            get_syscall_times_of(id) as isize
        }
        _ => -1,
    }
}

///
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    trace!("kernel: sys_mmap");
    // trace!("kernel: sys_mmap {:#x} ~ {:#x}", _start, _start + _len);
    let _len = (_len + 0xfff) & !0xfff;
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

///
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel: sys_munmap");
    if _len == 0 {
        return 0;//{ISSUE}
    }
    let _len = (_len + 0xfff) & !0xfff;
    //info!(">>>>>>>>>{:#x}!!{:#x}", _start, _start + _len);// any info/error! here will stuck system
    if do_task_munmap(_start, _len) { 0 } else { -1 }
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
