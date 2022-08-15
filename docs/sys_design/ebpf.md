# eBPF

## 设计思路

### 解释器

根据字节码，解析出立即数，偏移量，源寄存器，目标寄存器与操作码。

根据操作码，对寄存器进行操作

### eBPF helpers

结合rCore自身，设计helper函数

## 参考

[NickCao/ebpf-rs (github.com)](https://github.com/NickCao/ebpf-rs)

[linux bpf impl](https://elixir.bootlin.com/linux/v5.14.12/source/kernel/bpf) kernel/bpf  