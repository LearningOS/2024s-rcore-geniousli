# chapter5练习

# 问答作业： 
1. stride 算法深入
  1. 可能轮到 p1执行， p2.stride += 10 == 255 （数值溢出） 所以p1.stride == p2.stride， 具体下一个谁来执行 可能与位置以及调度算法相关
  2. 如果优先级>=2 则 每次进程执行之后增加的 step <= BigStride / 2。 调度算法为： 每次选取stride最小的，所以 stride_max - stride_min <= bigest step 所以 STRIDE_MAX – STRIDE_MIN <= BigStride / 2。
  3. 使用 TRIDE_MAX – STRIDE_MIN <= BigStride / 2
  ```rust
use core::cmp::Ordering;


struct Stride(u64);
pub const BIG_STRIDE: u64 = 10000;
pub const BIG_STEP: u64 = BIG_STRIDE / 2;

impl PartialOrd for Stride {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.0 == other.0 {
            return Some(Ordering::Equal);
        }
        if self.0 > other.0 {
            if self.0 - other.0 > BIG_STRIDE {
                return Some(Ordering::Less);
            }else {
                return Some(Ordering::Greater);
            }
        }else {
            if other.0 - self.0 > BIG_STRIDE {
                return Some(Ordering::Less);
            }else {
                return Some(Ordering::Greater);
            }
        }
    }
}

impl PartialEq for Stride {
    fn eq(&self, other: &Self) -> bool {
        false
    }
}

  ```
