use crate::ebpf::helper::HELPERS;
use alloc::collections::btree_map::BTreeMap;
use alloc::vec::Vec;
use core::cell::RefCell;
use ebpf_rs::interpret::interpret;
use lazy_static::*;
use trapframe::TrapFrame;

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
            alloc::boxed::Box::new(move |cx: &mut TrapFrame| {
                interpret(&prog, &HELPERS, cx as *const TrapFrame as usize as u64);
            }),
        )
    }
    pub fn disarm(&self) -> isize {
        crate::kprobes::kprobe_unregister(self.addr)
    }
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
