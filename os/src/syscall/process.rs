//! Process management syscalls
use crate::{
    config::MAX_SYSCALL_NUM,
    mm::translated_byte_buffer,
    task::{
        change_program_brk, current_user_token, exit_current_and_run_next, get_task_info,
        mmap_for_program, suspend_current_and_run_next, unmmap_for_program, TaskStatus,
    },
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    /// pass
    pub sec: usize,
    /// pass
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

impl TaskInfo {}

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
/// 1. TimeVal为 app中的虚拟地址， 系统调用后 已经进入到了 app 的kernel地址， 所以该ptr 指针应该需要进行翻译
/// 2. 跨页 同样应该进行处理， 比如 TimeVal [A, B],被划分为了page A，B。 A，B 存在不同的page， 进行不同的翻译.
/// 3. 为什么这里 可以直接使用ph address 呢？ 说明在 kernel时 使用的是 直接映射吗？ 不经过MMU？
/// 经过mmu， 不过 kernel对 其做了  直接映射， kernel将除了trapline 之外的所有内存做了直接映射ekernel ... memory_end. trapline 并不在 ekernle ... memory_end 之中，  所有其他的memory 分配都是 在下面这里 分配， 所以 kernel  可以直接access TimeVal
/// Physaddr::from(ekernel as usize).ceil(),
/// PhysAddr::from(MEMORY_END).floor(),

pub fn sys_get_time(its: *mut TimeVal, _tz: usize) -> isize {
    let us = crate::timer::get_time_us();
    let token = current_user_token();
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
}

/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(iti: *mut TaskInfo) -> isize {
    trace!("kernel: sys_task_info NOT IMPLEMENTED YET!");
    let src = get_task_info();
    let token = current_user_token();
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
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    // trace!("kernel: sys_mmap NOT IMPLEMENTED YET!");
    mmap_for_program(start, len, port)
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!("kernel: sys_unmmap NOT IMPLEMENTED YET!");
    unmmap_for_program(start, len)
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
