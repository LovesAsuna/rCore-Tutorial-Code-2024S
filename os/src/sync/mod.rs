//! Synchronization and interior mutability primitives

mod condvar;
mod mutex;
mod semaphore;
mod up;

use alloc::vec::Vec;
pub use condvar::Condvar;
pub use mutex::{Mutex, MutexBlocking, MutexSpin};
pub use semaphore::Semaphore;
pub use up::UPSafeCell;

use crate::task::{current_process, current_task};

/// Allocate available resources for deadlock detection
pub fn deadlock_detection_available_alloc(res_id: usize, res_count: u32) {
    ensure_capacity(
        &mut current_process()
            .inner_exclusive_access()
            .deadlock_detection_support
            .exclusive_access()
            .available,
        res_id + 1,
        0,
    )[res_id] += res_count;
}

/// Free available resources for deadlock detection
pub fn deadlock_detection_available_free(res_id: usize, res_count: u32) {
    ensure_capacity(
        &mut current_process()
            .inner_exclusive_access()
            .deadlock_detection_support
            .exclusive_access()
            .available,
        res_id + 1,
        0,
    )[res_id] -= res_count;
}

/// Allocate a allocation resource for deadlock detection
pub fn deadlock_detection_allocation_alloc(res_id: usize) {
    let tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let mut deadlock_detection_support =
        process_inner.deadlock_detection_support.exclusive_access();
    ensure_capacity(
        &mut ensure_capacity(
            &mut deadlock_detection_support.allocation,
            tid + 1,
            Vec::new(),
        )[tid],
        res_id + 1,
        0,
    )[res_id] += 1;
}

/// Free a allocation resource for deadlock detection
pub fn deadlock_detection_allocation_free(res_id: usize) {
    let tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let mut deadlock_detection_support =
        process_inner.deadlock_detection_support.exclusive_access();
    ensure_capacity(
        &mut ensure_capacity(
            &mut deadlock_detection_support.allocation,
            tid + 1,
            Vec::new(),
        )[tid],
        res_id + 1,
        0,
    )[res_id] -= 1;
}

/// Allocate a need resource for deadlock detection
pub fn deadlock_detection_need_alloc(res_id: usize) {
    let tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let mut deadlock_detection_support =
        process_inner.deadlock_detection_support.exclusive_access();
    ensure_capacity(
        &mut ensure_capacity(&mut deadlock_detection_support.need, tid + 1, Vec::new())[tid],
        res_id + 1,
        0,
    )[res_id] += 1;
}

/// Free a need resource for deadlock detection
pub fn deadlock_detection_need_free(res_id: usize) {
    let tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let mut deadlock_detection_support =
        process_inner.deadlock_detection_support.exclusive_access();
    ensure_capacity(
        &mut ensure_capacity(&mut deadlock_detection_support.need, tid + 1, Vec::new())[tid],
        res_id + 1,
        0,
    )[res_id] -= 1;
}

fn ensure_capacity<T>(vec: &mut Vec<T>, len: usize, value: T) -> &mut Vec<T>
where
    T: Clone,
{
    vec.resize(vec.len().max(len), value);
    vec
}
