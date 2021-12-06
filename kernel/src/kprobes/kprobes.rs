use crate::syscall;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use core::cell::RefCell;
use core::pin::Pin;
use lazy_static::*;
use trapframe::TrapFrame;

global_asm!(include_str!("ebreak.S"));
global_asm!(include_str!("jump.S"));

pub struct Kprobes {
    pub inner: RefCell<KprobesInner>,
}

pub struct KprobesInner {
    pub addr: usize,
    pub slot: Pin<Arc<[u8; 64]>>,
    pub pre_handler: fn(),
    pub insn_length: usize,
}

unsafe impl Sync for Kprobes {}

lazy_static! {
    pub static ref KPROBES: Kprobes = Kprobes::new();
}

extern "C" {
    pub fn __ebreak();
    pub fn __jump();
}

impl Kprobes {
    fn new() -> Self {
        Self {
            inner: RefCell::new(KprobesInner {
                addr: 0,
                pre_handler: || {},
                slot: Arc::pin([0; 64]),
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
        let addr = inner.addr;
        let addr_break = __ebreak as usize;
        let addr_jump = __jump as usize;
        let slot = &*inner.slot as *const [u8; 64] as usize;
        let temp = unsafe { core::slice::from_raw_parts_mut(addr as *mut u8, 1) };
        inner.insn_length = if temp[0] & 0b11 == 0b11 { 4 } else { 2 };
        let length = inner.insn_length;
        let mut addr = unsafe { core::slice::from_raw_parts_mut(addr as *mut u8, length) };
        let mut addr_break =
            unsafe { core::slice::from_raw_parts_mut(addr_break as *mut u8, length) };
        let mut addr_jump = unsafe { core::slice::from_raw_parts_mut(addr_jump as *mut u8, 4) };
        let mut slot_ptr = unsafe { core::slice::from_raw_parts_mut(slot as *mut u8, length) };
        let mut slot_ptr_next =
            unsafe { core::slice::from_raw_parts_mut((slot + length) as *mut u8, 4) };
        slot_ptr.copy_from_slice(addr);
        slot_ptr_next.copy_from_slice(addr_jump);
        addr.copy_from_slice(addr_break);
        unsafe { asm!("fence.i") };
    }
    pub fn unregister_kprobe(&self) {
        let inner = self.inner.borrow();
        let addr = inner.addr;
        let length = inner.insn_length;
        let mut addr = unsafe { core::slice::from_raw_parts_mut(addr as *mut u8, length) };
        addr.copy_from_slice(Pin::into_inner(inner.slot.clone()).as_ref());
    }
    fn kprobes_trap_handler(&self, cx: &mut TrapFrame) {
        let mut kprobes = self.inner.borrow_mut();
        if cx.sepc == kprobes.addr {
            (kprobes.pre_handler)();
            cx.general.s1 = cx.sepc + kprobes.insn_length; // TODO: encode s1 in jump instruction
            cx.sepc = &*kprobes.slot as *const [u8; 64] as usize;
        }
    }
}

pub fn kprobes_trap_handler(cx: &mut TrapFrame) {
    KPROBES.kprobes_trap_handler(cx);
}
