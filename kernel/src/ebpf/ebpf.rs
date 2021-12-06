use crate::ebpf::helper::HELPERS;
use alloc::collections::btree_map::BTreeMap;
use alloc::vec::Vec;
use core::cell::RefCell;
use ebpf_rs::interpret::interpret;
use trapframe::TrapFrame;

pub struct Ebpf {
    pub inner: RefCell<BTreeMap<usize, EbpfInner>>,
}

pub struct EbpfInner {
    addr: usize,
    prog: Vec<u64>,
}

impl EbpfInner {
    pub fn new(addr: usize, prog: Vec<u64>) -> Self {
        Self { addr, prog }
    }
    pub fn arm(&self) {
        let prog = self.prog.clone();
        crate::kprobes::kprobe_register(
            self.addr,
            alloc::boxed::Box::new(move |cx: &mut TrapFrame| {
                interpret(&prog, &HELPERS, cx as *const TrapFrame as usize as u64);
            }),
        );
    }
    pub fn disarm(&self) {
        crate::kprobes::kprobe_unregister(self.addr)
    }
}
