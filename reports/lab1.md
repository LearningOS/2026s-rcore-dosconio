# LAB1

## 实验总结

> 我实现了在例程的基础上，在**任务描述块**中增添了一个任务从初始化开始到主动问询自己状态期间，每个系统调用被访问过的次数 `syscall_times:[]`，和一个任务从第一次唤醒到问询当前“时间”  `start_time:isize` ，并完成了 `sys_task_info` 函数。



## 个人总结

> 笔者由于接触 `NASM/C`+`x86` 时间比较长，所以在 `Rust`+`RISCV64` 环境中感到了一点混乱。
>
> 笔者自己实现时，先是在任务上下文结构体里增添了 `syscall_times:[]` 和 `start_time:isize` ，然后觉得每次任务切换时都需要切换一次冗长的数据不合适，遂得灵感将两成员移入**任务描述块**，这似乎与 `x86` 中直接 `LTR` 或跳转调用 `TSS` 不同。
>
> 还有一点，由于不知道什么原因，项目一直构筑出错，可能是由于笔者的项目结构与例程不同导致的现象。



## 简答作业

1. 正确进入 U 态后，程序的特征还应有：使用 S 态特权指令，访问 S 态寄存器后会报错。 请同学们可以自行测试这些内容（运行 三个 bad 测例 (``ch2b_bad_*.rs`)）， 描述程序出错行为，同时注意注明你使用的 sbi 及其版本。

Implementation     : `RustSBI-QEMU Version 0.2.0-alpha.2` 

- `ch2b_bad_address`: 向 `0x00000000`写数据 - `[kernel] PageFault in application, bad addr = 0x0, bad instruction = 0x804003c4, kernel killed it.`
- `ch2b_bad_instructions`: 越级使用 `sret` 指令  -`[kernel] IllegalInstruction in application, kernel killed it.`
- `ch2b_bad_register`: 对 `sstatus` 进行操作 - `[kernel] IllegalInstruction in application, kernel killed it.`

2. 深入理解 `trap.S` 中两个函数 `__alltraps` 和 `__restore` 的作用，并回答如下问题:

（1）. L40：刚进入 `__restore` 时，`a0` 代表了什么值。请指出 `__restore` 的两种使用情景。

> 一种是 **Trap**，由 `__alltraps` 中 `call trap_handler` 调用返回后到此处执行。此时 `a0` 代表 函数 `trap_handler` 的返回值，即**任务上下文的地址**；
> 另一种是**任务切换**，由 `goto_restore` 函数设定，经过 `__switch` 之后便可等效地“恢复”目标任务的环境（入口、上下文等），事实上任务切换可以看作是**被动的 Trap**。 `a0` 为 `current_task_cx_ptr`，是执行权刚被撤离的任务的上下文；
> 体现了代码复用的思想，妙哉 OwO。



（2）. L43-L48：这几行汇编代码特殊处理了哪些寄存器？这些寄存器的的值对于进入用户态有何意义？请分别解释。

~~~assembly
```
ld t0, 32*8(sp)
ld t1, 33*8(sp)
ld t2, 2*8(sp)
csrw sstatus, t0
csrw sepc, t1
csrw sscratch, t2
```
~~~

- `t0` 用于暂存 `sstatus` 寄存器的值，`sstatus`为状态寄存器。
- `t1` 用于暂存 `spec` 寄存器的值，指向中断发生时的程序计数器/指令指针/引起中断的指令的地址，与 `x86` 不同，不要忘记移动到下一条指令，一般增加 `4` 即可。
- `t2` 用于暂存 `sscratch` 寄存器的值，指向用户栈栈顶，随后又指向内核栈栈顶（见`（4）`），用于换栈。

以上三个寄存器似乎不能由内存直接寻址赋值，需要经过通用寄存器中转一下。


（3）. L50-L56：为何跳过了 `x2` 和 `x4`？

```assembly
ld x1, 1*8(sp)
ld x3, 3*8(sp)
.set n, 5
.rept 27
   LOAD_GP %n
   .set n, n+1
.endr
```

- `x2`  地位相当于 `x86` 平台的 SP/BP 寄存器，指导栈的操作，后续还需要用栈所以不方便赋值，而且有 `sscratch` 暂存，最后两者互换即可。
- `x4` 应用程序一般不用这个寄存器。



（4）. L60：该指令之后，`sp` 和 `sscratch` 中的值分别有什么意义？

```assembly
csrrw sp, sscratch, sp
```

相当于 “`xchg sp, sscratch`”，之后 `sscratch` 指向内核栈栈顶，`sp` 转而指向用户栈，为进入用户态做准备。

（5）. `__restore`：中发生状态切换在哪一条指令？为何该指令执行之后会进入用户态？

`sret`

类似地，从 M 态进入之前状态使用 `mret`。

（6）. L13：该指令之后，`sp` 和 `sscratch` 中的值分别有什么意义？

```assembly
csrrw sp, sscratch, sp
```

相当于 “`xchg sp, sscratch`”，之后 `sscratch` 指向用户栈栈顶，暂存数据，`sp` 转而指向内核栈，为进入内核态做准备。


（7）. 从 U 态进入 S 态是哪一条指令发生的？

`ecall` 

## **荣誉准则**

1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 **以下各位** 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

    > 暂无

2. 此外，我也参考了 **以下资料** ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

    > https://learningos.cn/rCore-Tutorial-Guide-2024S/index.html （教材）

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。

