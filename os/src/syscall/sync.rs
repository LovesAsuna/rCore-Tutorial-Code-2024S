use crate::sync::{
    deadlock_detection_allocation_alloc, deadlock_detection_allocation_free,
    deadlock_detection_available_alloc, deadlock_detection_available_free,
    deadlock_detection_need_alloc, deadlock_detection_need_free, Condvar, Mutex, MutexBlocking,
    MutexSpin, Semaphore,
};
use crate::task::{block_current_and_run_next, current_process, current_task};
use crate::timer::{add_timer, get_time_ms};
use alloc::sync::Arc;
use alloc::vec::Vec;
/// sleep syscall
pub fn sys_sleep(ms: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_sleep",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let expire_ms = get_time_ms() + ms;
    let task = current_task().unwrap();
    add_timer(expire_ms, task);
    block_current_and_run_next();
    0
}
/// mutex create syscall
pub fn sys_mutex_create(blocking: bool) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mutex: Option<Arc<dyn Mutex>> = if !blocking {
        Some(Arc::new(MutexSpin::new()))
    } else {
        Some(Arc::new(MutexBlocking::new()))
    };
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .mutex_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.mutex_list[id] = mutex;
        id as isize
    } else {
        process_inner.mutex_list.push(mutex);
        process_inner.mutex_list.len() as isize - 1
    };
    drop(process_inner);
    drop(process);
    deadlock_detection_available_alloc(id.try_into().unwrap(), 1);
    id
}
/// mutex lock syscall
pub fn sys_mutex_lock(mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_lock",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );

    deadlock_detection_need_alloc(mutex_id);
    if detect_deadlock() {
        return -0xDEAD;
    }

    deadlock_detection_available_free(mutex_id, 1);
    deadlock_detection_allocation_alloc(mutex_id);
    deadlock_detection_need_free(mutex_id);

    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    drop(process);
    mutex.lock();
    0
}
/// mutex unlock syscall
pub fn sys_mutex_unlock(mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_unlock",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );

    deadlock_detection_available_alloc(mutex_id, 1);
    deadlock_detection_allocation_free(mutex_id);

    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    drop(process);
    mutex.unlock();
    0
}
/// semaphore create syscall
pub fn sys_semaphore_create(res_count: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .semaphore_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.semaphore_list[id] = Some(Arc::new(Semaphore::new(res_count)));
        id
    } else {
        process_inner
            .semaphore_list
            .push(Some(Arc::new(Semaphore::new(res_count))));
        process_inner.semaphore_list.len() - 1
    };
    drop(process_inner);
    drop(process);
    deadlock_detection_available_alloc(id, res_count as u32);
    id as isize
}
/// semaphore up syscall
pub fn sys_semaphore_up(sem_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_up",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    drop(process_inner);
    sem.up();

    deadlock_detection_available_alloc(sem_id, 1);
    deadlock_detection_allocation_free(sem_id);

    0
}
/// semaphore down syscall
pub fn sys_semaphore_down(sem_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_down",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );

    deadlock_detection_need_alloc(sem_id);
    if detect_deadlock() {
        return -0xDEAD;
    }

    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    drop(process_inner);
    sem.down();

    deadlock_detection_available_free(sem_id, 1);
    deadlock_detection_allocation_alloc(sem_id);
    deadlock_detection_need_free(sem_id);

    0
}
/// condvar create syscall
pub fn sys_condvar_create() -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .condvar_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.condvar_list[id] = Some(Arc::new(Condvar::new()));
        id
    } else {
        process_inner
            .condvar_list
            .push(Some(Arc::new(Condvar::new())));
        process_inner.condvar_list.len() - 1
    };
    id as isize
}
/// condvar signal syscall
pub fn sys_condvar_signal(condvar_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_signal",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    drop(process_inner);
    condvar.signal();
    0
}
/// condvar wait syscall
pub fn sys_condvar_wait(condvar_id: usize, mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_wait",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    condvar.wait(mutex);
    0
}
/// enable deadlock detection syscall
///
/// YOUR JOB: Implement deadlock detection, but might not all in this syscall
pub fn sys_enable_deadlock_detect(enabled: usize) -> isize {
    trace!("kernel: sys_enable_deadlock_detect");

    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();

    match enabled {
        0 => {
            process_inner.deadlock_detection = false;
        }
        1 => {
            process_inner.deadlock_detection = true;
        }
        _ => {
            return -1;
        }
    }

    0
}

fn detect_deadlock() -> bool {
    let process = current_process();
    let process_inner = process.inner_exclusive_access();

    if !process_inner.deadlock_detection {
        return false;
    }

    let process_deadlock_detection_support =
        process_inner.deadlock_detection_support.exclusive_access();

    let mut finish: Vec<bool> = Vec::new();
    finish.resize(process_inner.tasks.len().max(finish.len()), false);

    process_deadlock_detection_support
        .allocation
        .iter()
        .enumerate()
        .filter(|(_, alloc)| alloc.iter().all(|x| *x == 0))
        .for_each(|(idx, _)| {
            finish[idx] = true;
        });

    let mut work = process_deadlock_detection_support.available.clone();

    drop(process_deadlock_detection_support);
    drop(process_inner);
    drop(process);

    fn _find(finish: &Vec<bool>, work: &Vec<u32>) -> Option<usize> {
        let process = current_process();
        let process_inner = process.inner_exclusive_access();
        let process_deadlock_detection_support =
            process_inner.deadlock_detection_support.exclusive_access();
        process_inner
            .tasks
            .iter()
            .enumerate()
            .find(|(idx, _)| {
                !*finish.get(*idx).unwrap_or(&false)
                    && process_deadlock_detection_support
                        .need
                        .get(*idx)
                        .unwrap_or(&Vec::new())
                        .iter().enumerate().all(|(idx, x)| *x <= work[idx])
            })
            .map(|(idx, _)| idx)
    }

    let mut find = _find(&finish, &work);

    while let Some(idx) = find {
        let process = current_process();
        let process_inner = process.inner_exclusive_access();
        let process_deadlock_detection_support =
            process_inner.deadlock_detection_support.exclusive_access();
        process_deadlock_detection_support
                .allocation
                .get(idx)
                .unwrap_or(&Vec::new()).iter().enumerate().for_each(|(idx,x)| work[idx] += x);
        finish[idx] = true;
        drop(process_deadlock_detection_support);
        drop(process_inner);
        drop(process);
        find = _find(&finish, &work);
    }

    finish.iter().any(|it| !*it)
}
