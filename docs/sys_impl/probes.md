# 内核及用户态动态跟踪程序

- [内核跟踪库  rkprobes lib](#内核跟踪库-rkprobes_lib)
  - [使用说明](#使用说明)
    - [APIs](#APIs)
    - [方法](#方法)
- [rCore中内核及用户态跟踪程序](#rCore中内核及用户态跟踪程序)
  - [kprobes](#kprobes)
  - [uprobes](#uprobes)
    - [使用方法](#使用方法)

## 内核跟踪库-rkprobes_lib

可以在[kprobes的系统设计](../sys_design/kprobes.md)一节看到关于kprobes的实现原理.

如果单独使用kprobes，kprobes可以单独成为一个lib，方便所有操作系统调用.

lib: github[仓库.](https://github.com/hm1229/rkprobes)

### 使用说明

#### APIs

```rust
// register a kprobe, need the address of the function or instruction, two handler functions and the type you want to probe
pub fn kprobe_register(
    addr: usize, 
    handler: Arc<Mutex<dyn FnMut(&mut TrapFrame) + Send>>, 
    post_handler: Option<Arc<Mutex<dyn FnMut(&mut TrapFrame) + Send>>>, 
    probe_type: ProbeType
) -> isize ;

//unregister address-related probe
pub fn kprobe_unregister(addr: usize) -> isize;

//trap handler for handler kprobes
pub fn kprobes_trap_handler(cx: &mut TrapFrame);
```

#### 方法

- 讲 `kprobes_trap_handler`放到OS的中断处理程序中.例如：

  ```rust
  pub fn trap_handler_no_frame(tf: &mut TrapFrame) {
      let scause = scause::read();
      match scause.cause() {
          Trap::Exception(E::Breakpoint) => rkprobes::kprobes_trap_handler(tf), //add here
      }
  }
  ```

- 准备 `handler` 和 `post_handler`, `handler` 在函数或指令之前执行的跟踪函数， `post_hanlder` 在函数或指令执行之后运行的跟踪函数. `handler`是必须的,  `post_handler` 是可选择的,这两个函数的输入值是一个包含所有寄存器的结构体，使用的是TrapFrame的结构体.

  ```rust
  pub fn example_pre_handler(cx: &mut TrapFrame){
      println!{"pre_handler: spec:{:#x}", cx.sepc};
  }
  
  pub fn example_post_handler(cx: &mut TrapFrame){
      println!{"post_handler: spec:{:#x}", cx.sepc};
  }
  ```

- 为了注册一个 `kprobe`, 你需要传递函数或指令的地址 , 自己写好的`handler` 和 `post_handler`(可选), 跟踪的方式(函数或者指令).

  ```rust
  pub enum ProbeType{
      Insn,
      Func,
  }
  
  rkprobes::kprobe_register(
      self.addr,
      alloc::sync::Arc::new(Mutex::new(move |cx: &mut TrapFrame| {
          example_pre_handler(cx);
      })),
      Some(alloc::sync::Arc::new(Mutex::new(move |cx: &mut TrapFrame| {
          example_post_handler(cx);
      }))),
      ProbeType::Insn,
  )
  ```

- 为了注销一个`kprobe`, 你只需要传递跟踪点的地址.

  ```rust
  rkprobes::kprobe_unregister(addr)
  ```

## rCore中内核及用户态跟踪程序

### kprobes

kprobes的操作与[rkprobes](#使用说明)使用一致.

为了方便后续可以跟踪异步函数，probe_type结构体进行了一次更新.

```rust
pub enum ProbeType {
    Insn,
    SyncFunc,
    AsyncFunc,
}
```

### uprobes

在注册阶段需要额外传入需要跟踪的用户态进程的路径,其余参数与kprobes一样.

```rust
fn register_uprobes(
    &self,
    path: String,
    addr: usize,
    handler: Arc<Mutex<dyn FnMut(&mut UserContext) + Send>>,
    post_handler: Option<Arc<Mutex<dyn FnMut(&mut UserContext) + Send>>>,
    probe_type: ProbeType
) -> isize;
```

中断处理函数需要传入另一个存储寄存器的结构体

```rust
fn uprobes_trap_handler(&self, cx: &mut UserContext);
```

针对每一个用户态进程，需要在进入时初始化当前进程注册过的跟踪点

```rust
fn uprobes_init(&self)
```

#### 使用方法

如果需要使用uprobes，你需要：

- 注册uprobes.
- 将uprobes中断处理函数放到内核处理用户态发生的中断的函数中去.
- 在sys_exec中加入初始化函数`uprobes_init`.

