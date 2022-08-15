# 用Rust重新实现eBPF

## 项目描述

eBPF是Linux操作系统中监控和调试内核活动的方便工具，将eBPF以及其相关的kprobes内核监控程序与uprobes用户进程监控程序使用Rust进行重新实现，并针对Rust异步函数进行特有的监控.

该项目目前被应用于：

- [NickCao/rCore](https://github.com/NickCao/rCore)，其中kprobes作为独立的内核模块，提供代码调试等功能.
- [latte-c/rCore at bpf ](https://github.com/latte-c/rCore/tree/bpf)，其中kprobes与uprobes给eBPF提供跟踪点功能.

## 具体目标

ebpf in rust 目前几个主要目标

- [x] [riscv_insn_decode](./sys_impl/insn_decode.md)

  解码riscv指令，对其进行跟踪合法性分析.

- [x] [kprobes](./sys_impl/probes.md#内核跟踪库-rkprobes_lib)

  完成内核空间指令的动态插桩，对内核函数/合法指令进行跟踪.
  
- [x] [uprobes](./sys_impl/probes.md#uprobes)

  完成用户空间指令的动态插桩，对用户态程序中的函数/合法指令进行跟踪.
  
- [x] function parameter probing in probes

  完成probes对函数参数的获取.

- [x] [ebpf](./sys_impl/ebpf.md)

  根据ebpf原理实现一个简单的ebpf.

- [ ] async in probes

  完成probes对rust async 函数的跟踪支持.

