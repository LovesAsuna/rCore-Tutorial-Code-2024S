//! Process management syscalls
use alloc::sync::Arc;

use crate::{
   config, config::MAX_SYSCALL_NUM,
    loader::get_app_data_by_name,
    mm::{translated_refmut, translated_str},
    task::{
        add_task, current_task, current_user_token, exit_current_and_run_next,
        suspend_current_and_run_next, TaskStatus,
    }
};
use crate::task::TaskControlBlock;
use crate::mm::page_table::dereferencing_struct;
use crate::mm::{MapPermission, PageTable, VirtAddr, VirtPageNum};
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
pub fn sys_exit(exit_code: i32) -> ! {
    trace!("kernel:pid[{}] sys_exit", current_task().unwrap().pid.0);
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel:pid[{}] sys_yield", current_task().unwrap().pid.0);
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
    if let Some(data) = get_app_data_by_name(path.as_str()) {
        let task = current_task().unwrap();
        task.exec(data);
        0
    } else {
        -1
    }
}

/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    trace!("kernel::pid[{}] sys_waitpid [{}]", current_task().unwrap().pid.0, pid);
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

    trace!(
        "kernel:pid[{}] sys_get_time NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    0
}

/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(ti: *mut TaskInfo) -> isize {
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    let ms = get_time_ms();
    let info = TaskInfo {
        status: TaskStatus::Running,
        syscall_times: inner.syscall_times,
        time: ms - inner.start_time,
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
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    let memory_set = &mut inner.memory_set;
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

    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    let memory_set = &mut inner.memory_set;
    current_page = VirtAddr::from(start).floor();
    if !memory_set.delete_framed_area(VirtAddr::from(current_page.clone()), VirtAddr::from(end_page.clone())) {
        // println!("unmap failed");
        return -1;
    }
    0
}

/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel:pid[{}] sys_sbrk", current_task().unwrap().pid.0);
    if let Some(old_brk) = current_task().unwrap().change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}

/// YOUR JOB: Implement spawn.
/// HINT: fork + exec =/= spawn
pub fn sys_spawn(_path: *const u8) -> isize {
    trace!(
        "kernel:pid[{}] sys_spawn NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    -1
}

// YOUR JOB: Set task priority.
pub fn sys_set_priority(_prio: isize) -> isize {
    trace!(
        "kernel:pid[{}] sys_set_priority NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    -1
}
