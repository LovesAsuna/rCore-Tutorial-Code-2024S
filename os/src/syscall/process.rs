//! Process management syscalls
use crate::{config, config::MAX_SYSCALL_NUM, task::{
    change_program_brk, exit_current_and_run_next, suspend_current_and_run_next, TaskStatus,
}};
use crate::mm::page_table::dereferencing_struct;
use crate::mm::{MapPermission, PageTable, VirtAddr, VirtPageNum};
use crate::task::{current_user_token, TASK_MANAGER};
use crate::timer::{get_time_ms, get_time_us};

#[repr(C)]
#[derive(Debug, Clone)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// Task information
#[allow(dead_code)]
#[derive(Clone)]
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

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    let us = get_time_us();
    let tv = TimeVal {
        sec: us / 1_000_000,
        usec: us % 1_000_000,
    };

    dereferencing_struct(current_user_token(), ts as *const u8, tv);

    trace!("kernel: sys_get_time");
    0
}

/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(ti: *mut TaskInfo) -> isize {
    let inner = TASK_MANAGER.inner.exclusive_access();
    let tcb = &inner.tasks[inner.current_task];
    let ms = get_time_ms();
    let info = TaskInfo {
        status: TaskStatus::Running,
        syscall_times: tcb.syscall_times,
        time: ms - tcb.start_time,
    };
    drop(inner);
    dereferencing_struct(current_user_token(), ti as *const u8, info);

    0
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    // start 没有按页对齐
    if start & ((1 << config::PAGE_SIZE_BITS) - 1) != 0 {
        return -1;
    }
    // port 其余位必须为0
    if port & !0x7 != 0 {
        return -1;
    }
    // port 为 0，无意义内存
    if port & 0x7 == 0 {
        return -1;
    }
    // 拿到当前应用的页表
    let token = current_user_token();
    let page_table = PageTable::from_token(token);
    let mut current_page = VirtAddr::from(start).floor();
    let end_page = VirtAddr::from(start + len).ceil();
    while current_page.0 < end_page.0 {
        if let Some(entry) = page_table.translate(current_page) {
            if entry.is_valid() {
                // 存在已经被映射的页
                // println!("there is already mapped page {:?}", current_page);
                return -1;
            }
        }
        current_page = VirtPageNum(current_page.0 + 1);
    }
    let mut inner = TASK_MANAGER.inner.exclusive_access();
    let current_task = inner.current_task;
    let tcb = &mut inner.tasks[current_task];
    let memory_set = &mut tcb.memory_set;
    // 分配内存
    current_page = VirtAddr::from(start).floor();
    if !memory_set.insert_framed_area(VirtAddr::from(current_page), VirtAddr::from(end_page), MapPermission::from_bits((port as u8) << 1).unwrap() | MapPermission::U) {
        // 内存不足
        // println!("memory is not enough");
        return -1;
    }
    0
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    // start 没有按页对齐
    if start & ((1 << config::PAGE_SIZE_BITS) - 1) != 0 {
        return -1;
    }
    // 拿到当前应用的页表
    let token = current_user_token();
    let page_table = PageTable::from_token(token);
    let mut current_page = VirtAddr::from(start).floor();
    let end_page = VirtAddr::from(start + len).ceil();
    // println!("start: {:?}, end: {:?}", current_page, end_page);
    while current_page.0 < end_page.0 {
        if let None = page_table.translate(current_page) {
            // 存在未被映射的页
            // println!("there is unmapped page {:?}", current_page);
            return -1;
        }
        if let Some(entry) = page_table.translate(current_page)  {
            if !entry.is_valid() {
                // 存在无效的页
                // println!("there is invalid page {:?}", current_page);
                return -1;
            }
        }
        current_page = VirtPageNum(current_page.0 + 1);
    }

    let mut inner = TASK_MANAGER.inner.exclusive_access();
    let current_task = inner.current_task;
    let tcb = &mut inner.tasks[current_task];
    let memory_set = &mut tcb.memory_set;
    current_page = VirtAddr::from(start).floor();
    if !memory_set.delete_framed_area(VirtAddr::from(current_page.clone()), VirtAddr::from(end_page.clone())) {
        // println!("unmap failed");
        return -1;
    }
    0
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
