//! Types related to task management

use super::TaskContext;
use crate::config;
use crate::config::MAX_SYSCALL_NUM;
use crate::config::TRAP_CONTEXT_BASE;
use crate::mm::MapArea;
use crate::mm::{
    kernel_stack_position, MapPermission, MemorySet, PhysPageNum, VPNRange, VirtAddr, KERNEL_SPACE,
};
use crate::syscall::process::TaskInfo;
use crate::timer;
use crate::trap::{trap_handler, TrapContext};

/// The task control block (TCB) of a task Info
#[derive(Debug)]
pub struct TaskControlInfo {
    syscall_times: [u32; MAX_SYSCALL_NUM],
    time: Option<usize>,
}

/// The task control block (TCB) of a task.
pub struct TaskControlBlock {
    /// Save task context
    pub task_cx: TaskContext,

    /// Maintain the execution status of the current process
    pub task_status: TaskStatus,

    /// Application address space
    pub memory_set: MemorySet,

    /// The phys page number of trap context
    pub trap_cx_ppn: PhysPageNum,

    /// The size(top addr) of program which is loaded from elf file
    pub base_size: usize,

    /// Heap bottom
    pub heap_bottom: usize,

    /// Program break
    pub program_brk: usize,

    /// task_infos
    pub task_info: TaskControlInfo,
}

impl TaskControlBlock {
    /// get the trap context
    pub fn get_trap_cx(&self) -> &'static mut TrapContext {
        self.trap_cx_ppn.get_mut()
    }
    /// get the user token
    pub fn get_user_token(&self) -> usize {
        self.memory_set.token()
    }
    /// Based on the elf info in program, build the contents of task in a new address space
    /// 1. MemorySet::from_elf 直接将将app装在到os中，对bss,text deng做了映射
    /// 2. 分配对应的 trap context
    /// 3. kernel stack 是 app进入到kernel环境前的状态保存的地方
    pub fn new(elf_data: &[u8], app_id: usize) -> Self {
        // memory_set with elf program headers/trampoline/trap context/user stack
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT_BASE).into())
            .unwrap()
            .ppn();
        let task_status = TaskStatus::Ready;
        // map a kernel-stack in kernel space
        // top is bigger than bottom
        let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(app_id);
        KERNEL_SPACE.exclusive_access().insert_framed_area(
            kernel_stack_bottom.into(),
            kernel_stack_top.into(),
            MapPermission::R | MapPermission::W,
        );
        let task_control_block = Self {
            task_status,
            task_cx: TaskContext::goto_trap_return(kernel_stack_top),
            memory_set,
            trap_cx_ppn,
            base_size: user_sp,
            heap_bottom: user_sp,
            program_brk: user_sp,
            task_info: TaskControlInfo::default(),
        };
        // prepare TrapContext in user space
        let trap_cx = task_control_block.get_trap_cx();
        *trap_cx = TrapContext::app_init_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.exclusive_access().token(),
            kernel_stack_top,
            trap_handler as usize,
        );
        task_control_block
    }
    /// change the location of the program break. return None if failed.
    pub fn change_program_brk(&mut self, size: i32) -> Option<usize> {
        let old_break = self.program_brk;
        let new_brk = self.program_brk as isize + size as isize;
        if new_brk < self.heap_bottom as isize {
            return None;
        }
        let result = if size < 0 {
            self.memory_set
                .shrink_to(VirtAddr(self.heap_bottom), VirtAddr(new_brk as usize))
        } else {
            self.memory_set
                .append_to(VirtAddr(self.heap_bottom), VirtAddr(new_brk as usize))
        };
        if result {
            self.program_brk = new_brk as usize;
            Some(old_break)
        } else {
            None
        }
    }

    pub fn get_task_info(&self) -> TaskInfo {
        TaskInfo {
            status: self.task_status,
            syscall_times: self.task_info.syscall_times.clone(),
            time: timer::get_time_ms() - self.task_info.time.unwrap_or(0),
        }
    }

    /// mmap 分配的内存应该由 program管理，并且不在kernel中分配
    pub fn mmap(&mut self, start: usize, len: usize, port: usize) -> isize {
        let start_va: VirtAddr = start.into();
        if !start_va.aligned() || start <= config::MAXVA - len {
            return -1;
        }
        if let Some(pem) = MapPermission::convert_for_user(port) {
            // let start_va = VirtAddr::from(start).floor();
            // let end_va = VirtAddr::from(start + len).ceil();
            let (start_va, end_va) = VirtAddr::area_range(start, len);
            let map_area = crate::mm::MapArea::new_for_mmap(start_va, end_va, pem);
            if !self.memory_set.map_area_conflict(&map_area) {
                self.memory_set.push(map_area, None);
                return 0;
            }
        }

        return -1;
    }

    ///
    pub fn unmmap(&mut self, start: usize, len: usize) -> isize {
        // 让 start end 对齐
        let (start, end) = VirtAddr::area_range(start, len);
        let mut map = MapArea::new_for_unmap(start, end);
        if self.memory_set.unpush(&mut map) {
            return 0;
        }
        return -1;
    }
}

#[derive(Copy, Clone, PartialEq)]
/// task status: UnInit, Ready, Running, Exited
pub enum TaskStatus {
    /// uninitialized
    UnInit,
    /// ready to run
    Ready,
    /// running
    Running,
    /// exited
    Exited,
}
impl TaskControlInfo {
    pub fn incr_syscall_times(&mut self, syscall_id: usize) {
        self.syscall_times[syscall_id] += 1;
    }

    pub fn try_set_first_run_times(&mut self) {
        if self.time.is_none() {
            self.time = Some(crate::timer::get_time_ms())
        }
    }
}

impl Default for TaskControlInfo {
    fn default() -> Self {
        TaskControlInfo {
            syscall_times: [0; MAX_SYSCALL_NUM],
            time: None,
        }
    }
}
