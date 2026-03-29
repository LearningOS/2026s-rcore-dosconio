# LAB3

## 实验总结

> 我实现了在例程的基础上，实现了之前的功能，即将之前的分页内存模型例程兼容了本试验的 Multi-Process 模型，并实现了「spawn 系统调用」功能和「stride 调度算法」。


## 个人总结

> 由于出发点的不对，导致笔者在程序的编写、调试上花费了大量的时间。另外，笔者发现，如果 fork 或 “fexe” 一个程序不进行 wait 直接 exit，子程序不会自动时间片轮转，需要使用回车换行刺激触发。


## 简答作业 - stride 算法深入

stride 算法原理非常简单，但是有一个比较大的问题。例如两个 pass = 10 的进程，使用 8bit 无符号整形储存 stride， `p1.stride = 255`, `p2.stride = 250`，在 p2 执行一个时间片后，理论上下一次应该 p1 执行。

**实际情况是轮到 p1 执行吗？为什么？**

> 不是！如 TIPS， `255+10=265, 265%256=9`，很明显 `9<250`，所以还是 p1 执行，原因在于整型尺寸太小。

我们之前要求进程优先级 >= 2 其实就是为了解决这个问题。可以证明， 在不考虑溢出的情况下 , 在进程优先级全部 >= 2 的情况下，如果严格按照算法执行，那么 `STRIDE_MAX – STRIDE_MIN <= BigStride / 2`。

**为什么？尝试简单说明（不要求严格证明）。**

> 这意味着，即使发生回绕，也不太可能导致调度顺序的严重混乱。

**已知以上结论，考虑溢出的情况下，可以为 Stride 设计特别的比较器，让 `BinaryHeap<Stride>` 的 pop 方法能返回真正最小的 Stride。补全下列代码中的 partial_cmp 函数，假设两个 Stride 永远不会相等。**

```rust
use core::cmp::Ordering;  

struct Stride(u64);  

const PASS: u64 = 10;  

fn is_overflowed(a: u64, another: u64) -> bool {  
	a < another.wrapping_sub(PASS)  
}

impl PartialOrd for Stride {  
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {  
        let (self_val, other_val) = (self.0, other.0);  
        
        if is_overflowed(self_val, other_val) {  
            return Some(Ordering::Less);// self is less
        }  
        if is_overflowed(other_val, self_val) {  
            return Some(Ordering::Greater);// other is less
        }  
        self_val.partial_cmp(&other_val)  
    }  
}  
  
impl PartialEq for Stride {  
    fn eq(&self, other: &Self) -> bool {  
        false  
    }  
}
```

TIPS: 使用 8 bits 存储 stride, BigStride = 255, 则: `(125 < 255) == false`, `(129 < 255) == true`.

## 荣誉准则

1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 **以下各位** 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

    > 暂无

2. 此外，我也参考了 **以下资料** ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

    > https://learningos.cn/rCore-Tutorial-Guide-2024S/index.html （教材）

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。

