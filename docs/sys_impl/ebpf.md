# eBPF

- [解释器](#解释器)
  - [指令解码](#指令解码)
  - [指令的模拟执行](#指令的模拟执行)
    - [算数指令](#算数指令)
    - [跳转指令](#跳转指令)
- [eBPF helpers](#ebpf-helpers)

## 解释器

### 指令解码

通过pc指针，获取当前指令，将其拆分为立即数，偏移量，源寄存器，目标寄存器，操作码

```rust
let inst = insts[pc as usize];
pc += 1;
let imm: i32 = ((inst >> 32) & u32::MAX as u64) as i32;
let off: i16 = ((inst >> 16) & u16::MAX as u64) as i16;
let src: usize = ((inst >> 12) & 0x0f) as usize;
let dst: usize = ((inst >> 8) & 0x0f) as usize;
let op: u8 = (inst & u8::MAX as u64) as u8;
```

### 指令的模拟执行

由于eBPF指令较多，在此只展示部分指令的模拟执行过程.

#### 算数指令

> 根据源操作数的不同又分为：
>
> - BPF_K
>
>   使用 32 位即时作为源操作数
>
> - BPF_X
>
>   使用`src_reg`寄存器作为源操作数

- add

  ```rust
  ALU_K_ADD => reg[dst] = (reg[dst] as i32).wrapping_add(imm) as u64,
  ALU_X_ADD => reg[dst] = (reg[dst] as i32).wrapping_add(reg[src] as i32) as u64,
  ```

- or

  ```rust
  ALU_K_OR => reg[dst] = (reg[dst] as u32 | imm as u32) as u64,
  ALU_X_OR => reg[dst] = (reg[dst] as u32 | reg[src] as u32) as u64,
  ```

- etc.

#### 跳转指令

- call

  ```rust
  JMP_K_CALL => unsafe {
      reg[0] = helpers[imm as usize](reg[1], reg[2], reg[3], reg[4], reg[5]);
  },
  ```

- jlt

  ```rust
  JMP_K_JLT => {
      if reg[dst] < imm as u64 {
          pc = (pc as i16 + off) as u16;
      }
  }
  JMP_X_JLT => {
      if reg[dst] < reg[src] {
          pc = (pc as i16 + off) as u16;
      }
  }
  ```

- etc.

## eBPF-helpers

eBPF的帮助函数是与操作系统息息相关的，因为目前暂未实现map，所以只有少量帮助函数可用：

- `bpf_ktime_get_ns`

  打印当前时间

  ```rust
  // u64 bpf_ktime_get_ns(void)
  // return current ktime
  fn bpf_ktime_get_ns(_1: u64, _2: u64, _3: u64, _4: u64, _5: u64) -> u64 {
      crate::arch::timer::timer_now().as_nanos() as u64
  }
  ```

- `bpf_trace_printk`

  打印输出

  ```rust
  // long bpf_trace_printk(const char *fmt, u32 fmt_size, ...)
  unsafe fn bpf_trace_printk(fmt: u64, fmt_size: u64, p1: u64, p2: u64, p3: u64) -> u64 {
      let fmt = core::slice::from_raw_parts(fmt as *const u8, fmt_size as u32 as usize);
      println!(
          "{}",
          dyn_fmt::Arguments::new(core::str::from_utf8_unchecked(fmt), &[format!("{:#x}", p1), format!("{}", p2), format!("{}", p3)])
      );
      0
  }
  ```

- `bpf_get_current_pid_tgid`

  返回进程id

  ```rust
  fn bpf_get_current_pid_tgid(_1: u64, _2: u64, _3: u64, _4: u64, _5: u64) -> u64 {
      let thread = current_thread().unwrap();
      let pid = thread.proc.busy_lock().pid.get() as u64;
      // NOTE: tgid is the same with pid
      (pid << 32) | pid
  }
  ```

  
