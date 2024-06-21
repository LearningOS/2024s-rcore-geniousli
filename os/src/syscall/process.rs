//! Process management syscalls
use alloc::sync::Arc;
use crate::mm::translated_byte_buffer;

use crate::{
    config::MAX_SYSCALL_NUM,
    loader::get_app_data_by_name,
    mm::{translated_refmut, translated_str},
    task::{
        add_task, current_task, current_user_token, exit_current_and_run_next,
        suspend_current_and_run_next, TaskStatus,
    },
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
    pub status: TaskStatus,
    /// The numbers of syscall called by task
    pub syscall_times: [u32; MAX_SYSCALL_NUM],
    /// Total running time of task
    pub time: usize,
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
    // 不用sepc + 4 因为实在 sys_fork 调用之前 更改了 parent， child copy了他
    // 所以 child在fork之后的 启动位置在哪里？
    // child 执行的时候的位置应该在， trap_return, 因为执行 child的 control block是为 _switch 所以关键应该在 task_cx: 即：直接进入到内核态 ld ra 0(a1) (trap_return) ret
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
    trace!(
        "kernel::pid[{}] sys_waitpid [{}]",
        current_task().unwrap().pid.0,
        pid
    );
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
pub fn sys_get_time(its: *mut TimeVal, _tz: usize) -> isize {
    let us = crate::timer::get_time_us();
    if let Some(curr) = current_task() {
        let token = curr.get_user_token();
        let phy_dest = translated_byte_buffer(token, its as *const u8, core::mem::size_of::<TimeVal>());
        let src = TimeVal {
            sec: us / 1_000_000,
            usec: us % 1_000_000,
        };
        let src_ptr = &src as *const TimeVal;

        for (idx, dst) in phy_dest.into_iter().enumerate() {
            let len = dst.len();
            unsafe {
                dst.copy_from_slice(core::slice::from_raw_parts(
                    src_ptr.wrapping_byte_add(idx * len) as *const u8,
                    len,
                ));
            }
        }
        0
    }else {
        -1
    }
}

/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(iti: *mut TaskInfo) -> isize {
    if let Some(task) = current_task() {
        let src = task.get_task_info();
        let token = task.get_user_token();
        let phy_dest =
            translated_byte_buffer(token, iti as *const u8, core::mem::size_of::<TaskInfo>());
        let src_ptr = &src as *const TaskInfo;

        for (idx, dst) in phy_dest.into_iter().enumerate() {
            let len = dst.len();
            unsafe {
                dst.copy_from_slice(core::slice::from_raw_parts(
                    src_ptr.wrapping_byte_add(idx * len) as *const u8,
                    len,
                ));
            }
        }
        0
    } else {
        -1
    }
}

/// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    if let Some(curr) = current_task() {
        curr.mmap(start, len, port)
    } else {
        -1
    }
}

/// YOUR JOB: Implement munmap.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    if let Some(curr) = current_task() {
        curr.unmmap(start, len)
    } else {
        -1
    }
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
pub fn sys_spawn(path: *const u8) -> isize {
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(elf_data) = get_app_data_by_name(path.as_str()) {
        if let Some(task) = current_task() {
            let new_task = task.spawn(elf_data);
            let pid = new_task.pid.0 as isize;
            add_task(new_task);
            return pid;
        }
    }
    return -1;
}

// YOUR JOB: Set task priority.
pub fn sys_set_priority(prio: isize) -> isize {
    trace!(
        "kernel:pid[{}] sys_set_priority NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    if prio <= 1 {
        return -1;
    }
    if let Some(task) = current_task() {
        task.set_priority(prio);
        return prio;
    }
    -1
}
