use crate::syscall;
use alloc::sync::Arc;
use core::borrow::BorrowMut;
use core::cell::RefCell;
use core::pin::Pin;
use core::slice::from_raw_parts_mut;
use lazy_static::*;
use trapframe::TrapFrame;

pub struct Kprobes {
    pub inner: RefCell<KprobesInner>,
}

pub struct KprobesInner {
    pub addr: usize,
    pub slot: [u8; 8],
    pub pre_handler: fn(),
    pub insn_length: usize,
}

unsafe impl Sync for Kprobes {}

lazy_static! {
    pub static ref KPROBES: Kprobes = Kprobes::new();
}

#[naked]
extern "C" fn __ebreak() {
    unsafe {
        asm!("c.ebreak", "c.ebreak");
    }
}

impl Kprobes {
    fn new() -> Self {
        Self {
            inner: RefCell::new(KprobesInner {
                addr: 0,
                pre_handler: || {},
                slot: [0; 8],
                insn_length: 0,
            }),
        }
    }
    pub fn register_kprobe(&self, pre_handler: fn()) {
        let mut inner = self.inner.borrow_mut();
        let addr = syscall::hook_point as usize;
        inner.addr = addr;
        inner.pre_handler = pre_handler;
        drop(inner);
        self.prepare_kprobe();
    }
    pub fn prepare_kprobe(&self) {
        let mut inner = self.inner.borrow_mut();
        // read the lowest byte of the probed instruction to determine whether it is compressed
        inner.insn_length = if unsafe { *(inner.addr as *const u8) } & 0b11 == 0b11 {
            4
        } else {
            2
        };
        // TODO: check whether the instruction is safe to execute out of context
        let mut addr = unsafe { from_raw_parts_mut(inner.addr as *mut u8, inner.insn_length) };
        let mut addr_break = unsafe { from_raw_parts_mut(__ebreak as *mut u8, inner.insn_length) };
        // save the probed instruction to a buffer
        inner.slot[..length].copy_from_slice(addr);
        // append ebreak to the buffer
        inner.slot[length..length + length].copy_from_slice(addr_break);
        // replace the instruction with ebreak
        addr.copy_from_slice(addr_break);
        unsafe { asm!("fence.i") };
    }
    pub fn unregister_kprobe(&self) {
        let inner = self.inner.borrow();
        unsafe {
            from_raw_parts_mut(inner.addr as *mut u8, inner.insn_length)
                .copy_from_slice(&inner.slot);
            asm!("fence.i")
        };
    }
    fn kprobes_trap_handler(&self, cx: &mut TrapFrame) {
        let mut kprobes = self.inner.borrow_mut();
        if cx.sepc == kprobes.addr {
            (kprobes.pre_handler)();
            cx.sepc = &kprobes.slot as *const [u8; 8] as usize;
        } else {
            cx.sepc = kprobes.addr + kprobes.insn_length;
        }
    }
}

pub fn kprobes_trap_handler(cx: &mut TrapFrame) {
    KPROBES.kprobes_trap_handler(cx);
}
