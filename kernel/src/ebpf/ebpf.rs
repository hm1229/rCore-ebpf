use crate::ebpf::helper::HELPERS;
use alloc::collections::btree_map::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use num::integer;
use num_traits::int;
use core::cell::RefCell;
use ebpf_rs::interpret::interpret;
use lazy_static::*;
use spin::Mutex;
use trapframe::{TrapFrame, UserContext};
use crate::kprobes::{ProbeType, uprobe_register, kprobe_register, kprobe_unregister,ProbePlace};
use riscv::register::*;
use core::{
    future::Future,
    mem::MaybeUninit,
    pin::Pin,
    task::{Context, Poll},
};
use crate::lkm::manager::ModuleManager;
use executor;
use riscv::register::mcause::Trap;

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

fn resolve_symbol(symbol: &str) -> Option<usize> {
    ModuleManager::with(|mm| mm.resolve_symbol(symbol))
}

impl EbpfInner {
    pub fn new(addr: usize, prog: Vec<u64>) -> Self {
        Self { addr, prog }
    }
    pub fn arm(&self, path: String, pp: ProbePlace) -> isize {
        let prog = self.prog.clone();
        match pp {
            ProbePlace::Kernel(ProbeType::Insn) => {
                kprobe_register(
                    self.addr,
                    alloc::sync::Arc::new(Mutex::new(move |cx: &mut TrapFrame| {
                        interpret(&prog, &HELPERS, cx as *const TrapFrame as usize as u64);
                    })),
                    Some(alloc::sync::Arc::new(Mutex::new(move |cx: &mut TrapFrame| {
                        test_kernel_post_handler(cx);
                    }))),
                    ProbeType::Insn
                )
            }
            ProbePlace::Kernel(ProbeType::SyncFunc) => {
                kprobe_register(
                    self.addr,
                    alloc::sync::Arc::new(Mutex::new(move |cx: &mut TrapFrame| {
                        interpret(&prog, &HELPERS, cx as *const TrapFrame as usize as u64);
                    })),
                    Some(alloc::sync::Arc::new(Mutex::new(move |cx: &mut TrapFrame| {
                        test_kernel_post_handler(cx);
                    }))),
                    ProbeType::SyncFunc
                )
            }
            ProbePlace::User(ProbeType::Insn) => {
                uprobe_register(
                    path,
                    self.addr,
                    alloc::sync::Arc::new(Mutex::new(move |cx: &mut UserContext| {
                        interpret(&prog, &HELPERS, cx as *const UserContext as usize as u64);
                    })),
                    Some(alloc::sync::Arc::new(Mutex::new(move |cx: &mut UserContext| {
                        test_post_handler(cx);
                    }))),
                    ProbeType::Insn
                )
            }
            ProbePlace::User(ProbeType::SyncFunc) => {
                uprobe_register(
                    path,
                    self.addr,
                    alloc::sync::Arc::new(Mutex::new(move |cx: &mut UserContext| {
                        interpret(&prog, &HELPERS, cx as *const UserContext as usize as u64);
                    })),
                    Some(alloc::sync::Arc::new(Mutex::new(move |cx: &mut UserContext| {
                        test_post_handler(cx);
                    }))),
                    ProbeType::SyncFunc
                )
            }
            _ => {
                -1
            }
        }
    }
    pub fn disarm(&self) -> isize {
        kprobe_unregister(self.addr)
    }
}

pub fn test_pre_handler(cx: &mut UserContext){
    println!{"pre_handler: spec:{:#x}", cx.sepc};
}

pub fn test_post_handler(cx: &mut UserContext){
    println!{"post_handler: spec:{:#x}", cx.sepc};
}

pub fn test_kernel_pre_handler(cx: &mut TrapFrame){
    println!{"pre_handler: spec:{:#x}", cx.sepc};
}

pub fn test_kernel_post_handler(cx: &mut TrapFrame){
    println!{"post_handler: spec:{:#x}", cx.sepc};
}



impl Ebpf {
    pub fn new() -> Self {
        Self {
            inner: RefCell::new(BTreeMap::new()),
        }
    }
    pub fn register(&self, addr: usize, prog: Vec<u64>, path: String, pp: ProbePlace) -> isize {
        let ebpf = EbpfInner::new(addr, prog);
        let ret = ebpf.arm(path, pp);
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

// before function: 
#[inline(never)]
pub async fn test_async(){
    executor::spawn(test1_async());
    println!("in_async");
    test2_async().await;
}
// after function: 


async fn test1_async(){
    for i in 1..=5{
        let buffer: u64 = yield_now().await;
        println!("test1 {}---------{}", i, buffer);
        // yield_now().await;
    }
}

async fn test2_async(){
    for i in 1..=5{
        let buffer: u64 = yield_now().await;
        println!("test2 {}---------{}", i, buffer);
        // yield_now().await;
    }
}

pub fn yield_now() -> impl Future<Output = u64> {
    YieldFuture{
        time: get_time_ms(),
    }
}

#[derive(Default)]
struct YieldFuture {
    time: usize,
}

impl Future for YieldFuture {
    type Output = u64;

    #[inline(never)]
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        // println!("time  {}", self.time);
        if get_time_ms() - self.time > 5000 {
            // println!("ready!!!");
            Poll::Ready(10)
        } else {
            // self.flag = true;
            cx.waker().clone().wake();
            Poll::Pending
        }
    }
}
