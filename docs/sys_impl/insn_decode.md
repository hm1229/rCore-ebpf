# 指令解码

根据Riscv设计文档进行解码.

- [指令合法性判断库](#指令合法性判断库)
  - [使用说明](#使用说明)
    - [数据结构](#数据结构)
      - [InsnStatus](#InsnStatus)
    - [APIs](#APIs)
      - [get_insn_length](#get_insn_length)
      - [insn_decode](#insn_decode)
- [sp相关指令解码](#sp相关指令解码)
  - [addi](#addi)
  - [c.addi16sp](#c.addi16sp)
  - [c.addi](#c.addi)
  - [c.addi4spn](#c.addi4spn)

## 指令合法性判断库

在已有的riscv-decode库的基础上，增加了压缩指令的解码过程

将riscv_insn_decode代码解耦合后，独立成库:[riscv_insn_decode](https://github.com/hm1229/riscv_insn_decode)

### 使用说明

如果要使用 `riscv_insn_decode`库,首先需要更新 `Cargo.toml`

```toml
[dependencies]
riscv_insn_decode = {git = "https://github.com/hm1229/riscv_insn_decode", rev = "0b954c9"}
```

#### 数据结构

##### InsnStatus

指令是否合法

```rust
#[derive(Debug)]
pub enum InsnStatus {
    Illegal,
    Legal,
}
```

#### APIs

##### get_insn_length

根据传入的指令地址，返回指令的长度

```rust
use riscv_insn_decode::get_insn_length;

let length = get_insn_length(addr);
```

##### insn_decode

根据传入的指令地址，返回指令是否合法.

```rust
use riscv_insn_decode::{insn_decode, InsnStatus};

match insn_decode(addr){
    InsnStatus::Legal => {
        unimplemented!();
    },
    InsnStatus::Illegal => {
        unimplemented!();
    }
}
```

## sp相关指令解码

根据Riscv文档，解析立即数.

### addi

```rust
let inst = u32::from_le_bytes(slot[..4].try_into().unwrap());

// addi sp, sp, imm
let addisp = sext(((inst >> 20) & 0b111111111111) as isize, 12) as usize;
```

### c.addi16sp imm

```rust
fn sext(x: isize, size: usize) -> isize {
    let shift = core::mem::size_of::<isize>() * 8 - size;
    (x << shift) >> shift
}

let inst = u16::from_le_bytes(slot[..2].try_into().unwrap());
let addisp = sext(
     ((((inst >> 12) & 0b1) << 9)
         + (((inst >> 6) & 0b1) << 4)
         + (((inst >> 5) & 0b1) << 6)
         + (((inst >> 3) & 0b11) << 7)
         + (((inst >> 2) & 0b1) << 5)) as isize,
     10,
) as usize;
```

### c.addi sp, imm

```rust
fn sext(x: isize, size: usize) -> isize {
    let shift = core::mem::size_of::<isize>() * 8 - size;
    (x << shift) >> shift
}

let inst = u16::from_le_bytes(slot[..2].try_into().unwrap());
let addisp = sext(
    ((((inst >> 12) & 0b1) << 5) + (((inst >> 2) & 0b11111) << 0)) as isize,
    6,
) as usize;
```

### c.addi4spn

```rust
fn sext(x: isize, size: usize) -> isize {
    let shift = core::mem::size_of::<isize>() * 8 - size;
    (x << shift) >> shift
}

let inst = u16::from_le_bytes(slot[..2].try_into().unwrap());
let addisp = sext(((((inst >> 11) & 0b111) << 3)
    + (((inst >> 7) & 0b1111) << 5)
    + (((inst >> 6) & 0b1) << 1)
    + (((inst >> 5) & 0b1) << 2)) as isize,
    10
) as usize;
```

