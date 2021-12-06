use crate::syscall;
use alloc::collections::btree_map::BTreeMap;
use alloc::sync::Arc;
use core::borrow::BorrowMut;
use core::cell::RefCell;
use core::pin::Pin;
use core::slice::{from_raw_parts, from_raw_parts_mut};
use lazy_static::*;
use trapframe::TrapFrame;

pub struct Kprobes {
    pub inner: RefCell<BTreeMap<usize, KprobesInner>>,
    pub ret: RefCell<BTreeMap<usize, usize>>,
}

pub struct KprobesInner {
    pub addr: usize,
    pub length: usize,
    pub slot: [u8; 8],
    pub handler: fn(&mut TrapFrame),
}

unsafe impl Sync for Kprobes {}
unsafe impl Sync for KprobesInner {}

lazy_static! {
    pub static ref KPROBES: Kprobes = Kprobes::new();
}

#[naked]
extern "C" fn __ebreak() {
    unsafe {
        asm!("c.ebreak", "c.ebreak");
    }
}

impl KprobesInner {
    pub fn new(addr: usize, handler: fn(&mut TrapFrame)) -> Self {
        // read the lowest byte of the probed instruction to determine whether it is compressed
        let length = if unsafe { *(addr as *const u8) } & 0b11 == 0b11 {
            4
        } else {
            2
        };
        // TODO: check whether the instruction is safe to execute out of context
        let mut inst = unsafe { from_raw_parts_mut(addr as *mut u8, length) };
        let mut slot = [0; 8];
        // save the probed instruction to a buffer
        slot[..length].copy_from_slice(inst);
        // append ebreak to the buffer
        let ebreak = unsafe { from_raw_parts(__ebreak as *const u8, length) };
        slot[length..length + length].copy_from_slice(ebreak);
        Self {
            addr,
            length,
            slot,
            handler,
        }
    }
    pub fn arm(&self) {
        // replace the probed instruction with ebreak
        let ebreak = unsafe { from_raw_parts(__ebreak as *const u8, self.length) };
        let mut inst = unsafe { from_raw_parts_mut(self.addr as *mut u8, self.length) };
        inst.copy_from_slice(ebreak);
        unsafe { asm!("fence.i") };
    }
    pub fn disarm(&self) {
        let mut inst = unsafe { from_raw_parts_mut(self.addr as *mut u8, self.length) };
        inst.copy_from_slice(&self.slot);
        unsafe { asm!("fence.i") };
    }
}

impl Kprobes {
    pub fn new() -> Self {
        Self {
            inner: RefCell::new(BTreeMap::new()),
            ret: RefCell::new(BTreeMap::new()),
        }
    }
    pub fn register_kprobe(&self, addr: usize, handler: fn(&mut TrapFrame)) -> isize {
        let probe = KprobesInner::new(addr, handler);
        probe.arm();
        if let Some(replaced) = self.inner.borrow_mut().insert(addr, probe) {
            replaced.disarm();
        }
        0
    }
    pub fn unregister_kprobe(&self, addr: usize) -> isize {
        if let Some(probe) = self.inner.borrow_mut().remove(&addr) {
            probe.disarm();
            return 0;
        }
        -1
    }
    pub fn kprobes_trap_handler(&self, cx: &mut TrapFrame) {
        let mut kprobes = self.inner.borrow_mut();
        match kprobes.get(&cx.sepc) {
            Some(probe) => {
                (probe.handler)(cx);
                let slot = &probe.slot as *const [u8; 8] as usize;
                cx.sepc = slot;
                self.ret
                    .borrow_mut()
                    .insert(slot + probe.length, probe.addr + probe.length);
            }
            None => {
                if let Some(addr) = self.ret.borrow_mut().remove(&cx.sepc) {
                    cx.sepc = addr;
                }
            }
        }
    }
}

pub fn kprobes_trap_handler(cx: &mut TrapFrame) {
    KPROBES.kprobes_trap_handler(cx);
}
