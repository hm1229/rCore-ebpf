# Rust 异步并发

## 简介

在当前的 rust 生态中，我们可以使用async/await来将异步函数按照同步函数的格式一样书写，使代码更加整洁，易于维护。

## Future并发模式

Future异步并发模式是以代理模式和异步开发的混合产物，future是对将来的一种代理凭证，凭借这个凭证可以异步地在未来某个时刻得到确定的结果。

Rust对Future异步并发模式做了一个完整的抽象，包含在第三方库`future-rs`中。该抽象主要包含三个部件：

- **Future**： 基本的异步计算抽象单元
- **Executor**：异步计算调度层
- **Task**：异步计算执行层

### Future

在Rust中Future是一个trait，其源代码为：

```rust
pub trait Future{
    type Output;
    fn poll(self: Pin<&mut self>, lw: &LocalWaker) -> Poll<Self::Output>;
}
```

其中poll方法是Future的核心，它是对轮询行为的一种抽象。在Rust中，每个Future都需要使用poll方法来轮询所要计算值的状态。该方法返回的Poll是一个枚举类型：

```rust
pub enum Poll<T>{
    Ready(T),
    Pending,
}
```

Poll\<T>枚举类型是对准备好和未完成两种状态的统一抽象，以此来表达Future的结果。

### Executor与Task

在实际的异步开发中，需要一个专门的调度器来对具体的任务进行管理统筹，这个调度器就是Executor，具体的异步任务就是Task。

## async/await

async：产生一个 Future 对象，一个没有任何作用的对象，必须由调用器调用才会有用。
await: 等待异步操作完成（基于语义理解，其实很多情况只有调用 future.await 才是事实上去调用，具体是不是之前就开始执行，这个要看我们的调用器是什么），这步是阻塞当前线程，这个语法属于 Future 对象才能调用，而且必须要在 async 函数内。

## 具体实现原理

async块会生成一个Generator\<Yield=()>的生成器，然后将该生成器通过单元结构体GenFuture进行包装，最后为该GenFuture实现Future

await!展开的代码会在loop循环中进行判断，如果是Ready则退出。