# chapter3练习

## 实现的功能有：
1. 在 TaskManager 中添加 struct TaskExtInfo 主要用于记录 syscall_times 以及 第一次运行时间的时间戳， 
2. 在第一次运行时，设定  task_ext_info 中的 time, 之后调用 判断 time为None，不进行设置。
3. 在每次 syscall时候，首先进行 add_syscall_times 调用， 使用 syscall_id 作为index 将 syscall_times 对应位置 + 1

# 问答作业：
1. 
2. 
2.1 L40: __restore 并没有被直接调用， 进入__restore 的形式为： __alltraps 在 call trap_handler这条命令 （call rd symbol, 将 pc+8(__restore) 写入 x[rd], pc = symbol, rd忽略的话，默认为x1(ra), 即： 在执行完 symbol后 跳转到 call rd symbol 下一条命令执行（__restore））, 函数调用顺序为： alltraps -> trap_handler -> restore 在进入restore 函数时候 寄存器 a0 取决于 trap_handler 最后的函数调用。
使用场景有： 
  1. 恢复 app 进入 kernel时候保存的 TrapContext，进行特权级切换。
  2. trap 时候， _alltraps 保存 TrapContext， 执行完毕之后， _restore 进行恢复
  3. trap 时候， app切换时（调用 switch） ，_restore 恢复到 另外一个 app的状态
2.2. sstatus, sepc. sscratch。 sstatus： 为状态寄存器, 具有控制中断， sepc 控制在sret 之后pc的设定， sscratch 为app的 stack sp
2.3 x2(sp) 栈指针 x4(tp) 线程指针
2.4 sp 与 sscratch 进行互换。 sp 指向 app stack， sscratch 为 TapContext 起始位置 
2.5 ssret 命令， mret命令会在懂 将 将ssttus中的 mode 设定为mpp。 
2.6 sscratch为 user stack, sp 指向 TrapContext, sscratch 在什么时候设定? 应该是 机器设定吗？
2.7 ecall指令发生
