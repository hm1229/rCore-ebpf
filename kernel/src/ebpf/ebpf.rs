use crate::ebpf::helper::HELPERS;
use alloc::collections::btree_map::BTreeMap;
use alloc::vec::Vec;
use core::cell::RefCell;
use ebpf_rs::interpret::interpret;
use lazy_static::*;
use spin::Mutex;
use trapframe::TrapFrame;
use crate::kprobes::{kprobe_register, ProbeType};
use riscv::register::*;
use core::{
    future::Future,
    mem::MaybeUninit,
    pin::Pin,
    task::{Context, Poll},
};

pub struct Ebpf {
    pub inner: RefCell<BTreeMap<usize, EbpfInner>>,
}

pub struct EbpfInner {
    addr: usize,
    prog: Vec<u64>,
}

unsafe impl Sync for Ebpf {}
unsafe impl Sync for EbpfInner {}

lazy_static! {
    pub static ref EBPF: Ebpf = Ebpf::new();
}

impl EbpfInner {
    pub fn new(addr: usize, prog: Vec<u64>) -> Self {
        Self { addr, prog }
    }
    pub fn arm(&self) -> isize {
        let prog = self.prog.clone();
        crate::kprobes::kprobe_register(
            self.addr,
            // alloc::sync::Arc::new(Mutex::new(move |cx: &mut TrapFrame| {
            //     interpret(&prog, &HELPERS, cx as *const TrapFrame as usize as u64);
            // })),
            // None,
            alloc::sync::Arc::new(Mutex::new(move |cx: &mut TrapFrame| {
                test_pre_handler(cx);
            })),
            Some(alloc::sync::Arc::new(Mutex::new(move |cx: &mut TrapFrame| {
                test_post_handler(cx);
            }))),
            ProbeType::insn,
        )
    }
    pub fn disarm(&self) -> isize {
        crate::kprobes::kprobe_unregister(self.addr)
    }
}

pub fn test_pre_handler(cx: &mut TrapFrame){
    println!{"pre_handler: spec:{:#x}", cx.sepc};
}

pub fn test_post_handler(cx: &mut TrapFrame){
    println!{"post_handler: spec:{:#x}", cx.sepc};
}


impl Ebpf {
    pub fn new() -> Self {
        Self {
            inner: RefCell::new(BTreeMap::new()),
        }
    }
    pub fn register(&self, addr: usize, prog: Vec<u64>) -> isize {
        let ebpf = EbpfInner::new(addr, prog);
        let ret = ebpf.arm();
        if ret != 0 {
            return ret;
        }
        if let Some(replaced) = self.inner.borrow_mut().insert(addr, ebpf) {
            replaced.disarm();
        }
        0
    }
    pub fn unregister(&self, addr: usize) -> isize {
        if let Some(ebpf) = self.inner.borrow_mut().remove(&addr) {
            ebpf.disarm();
            return 0;
        }
        -1
    }
}
fn get_time_ms() -> usize{
    // println!("get_time, {}", time::read() * 62 * 1000 / 403000000);
    time::read() * 62 * 1000 / 403000000
}

pub async fn test_async(){
    for i in 1..=5{
        println!("test {}", i);
        yield_now().await;
    }
}

pub fn yield_now() -> impl Future<Output = ()> {
    YieldFuture{
        time: get_time_ms(),
    }
}

#[derive(Default)]
struct YieldFuture {
    time: usize,
}

impl Future for YieldFuture {
    type Output = ();

    #[inline(never)]
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        // println!("time  {}", self.time);
        if get_time_ms() - self.time >5000 {
            // println!("ready!!!");
            Poll::Ready(())
        } else {
            // self.flag = true;
            cx.waker().clone().wake();
            Poll::Pending
        }
    }
}
