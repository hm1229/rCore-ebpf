# 开发日志

### 8.15

继续尝试根据符号表信息查找函数地址，发现需要修改虚拟分页等一系列代码，暂时搁置。

内核函数插桩恢复为地址插桩。

### 8.14

重构用户态演示程序，使其支持多种类型的跟踪。

重构ebpf::arm程序与演示程序对接。

尝试通过rCore内已有的符号表获取模块实现通过函数名查找函数地址的功能，没成功。

### 8.13

完成demo ebpf测试程序。

测试demo.c，ebpfrv解释器可以正常解析并运行

### 8.12

使用小型的ebpfrv作为代替

完成k/uprobes的代码重构，整合重复代码，使接口更为易用

### 8.11

错误详情：

- 在user下用`b.sh`

  ![image-20220811145245415](C:\Users\qq171\AppData\Roaming\Typora\typora-user-images\image-20220811145245415.png)

- 启动kernel`make run ARCH=riscv64 LOG=warn`

- 在kernel中启动`./busybox`可以正常运行

  ![image-20220811151103255](C:\Users\qq171\AppData\Roaming\Typora\typora-user-images\image-20220811151103255.png)

- 运行`./bmonitor`就会报页错误：

  ![image-20220811151133996](C:\Users\qq171\AppData\Roaming\Typora\typora-user-images\image-20220811151133996.png)

### 8.8

排查bmonitor启动失败的原因

- 报错：panicked at 'page fault handle failed'
- 尝试启动其它用户态程序：可以正常启动
- 只有bmonitor与bpf-test启动时会报错误

### 7.30

bmonitor启动不起来

迁移方案：

- kprobe->kprobe:insn
- kretprobe->kprobe:syncfunc

新增

- uprobe:insn
- uprobe:syncfunc

### 7.23

完成ebpf模块

针对不同实现方法的kprobe进行设计

### 5.31

完成初赛技术报告.

### 5.29

完成完整的uprobes功能，可以动态跟踪任意用户进程.

### 5.15

完成部分uprobes功能，可以动态跟踪当前用户进程的函数或指令.

### 1.31

完成kprobes相关功能的实现，内核相关程序可以动态进行插桩.

