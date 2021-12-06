use crate::syscall;
use alloc::string::{String, ToString};
use core::cell::RefCell;
use lazy_static::*;
use trapframe::TrapFrame;

global_asm!(include_str!("ebreak.S"));
global_asm!(include_str!("jump.S"));

pub struct Kprobes {
    pub inner: RefCell<KprobesInner>,
}

pub struct KprobesInner {
    pub addr: usize,

    pub name: String,

    /* 在被探测点指令执行之前调用的回调函数 */
    pub pre_handler: fn(),

    /* 在被探测点指令执行之后调用的回调函数 */
    pub post_handler: fn(),

    /* 被复制的被探测点的原始指令（Risc-V） */
    pub insn_back: usize,

    pub slot_addr: usize,

    pub insn_s1: usize,

    pub insn_length: usize,
}
unsafe impl Sync for Kprobes {}

lazy_static! {
    pub static ref KPROBES: Kprobes = Kprobes::new();
}

extern "C" {
    pub fn __ebreak();
}
extern "C" {
    pub fn __jump();
}

impl Kprobes {
    /* 向内核注册kprobe探测点 */
    fn new() -> Self {
        Self {
            inner: RefCell::new(KprobesInner {
                addr: 0,
                name: "".to_string(),
                pre_handler: slot_insn,
                post_handler: slot_insn,
                insn_back: 0,
                slot_addr: 0,
                insn_s1: 0,
                insn_length: 0,
            }),
        }
    }
    pub fn register_kprobe(&self, pre_handler: fn(), post_handler: fn()) {
        let mut inner = self.inner.borrow_mut();
        let addr = syscall::hook_point as usize;
        inner.addr = addr;
        inner.pre_handler = pre_handler;
        inner.post_handler = post_handler;
        inner.slot_addr = slot_insn as usize;
        drop(inner);
        self.prepare_kprobe();
    }
    pub fn prepare_kprobe(&self) {
        let addr = self.inner.borrow().addr;
        let addr_break = __ebreak as usize;
        let addr_jump = __jump as usize;
        let slot = slot_insn as usize;

        let temp = unsafe { core::slice::from_raw_parts_mut(addr as *mut u8, 1) };
        if temp[0] & 0b11 == 3 {
            self.inner.borrow_mut().insn_length = 4;
        } else {
            self.inner.borrow_mut().insn_length = 2;
        }
        let length = self.inner.borrow().insn_length;
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
        let addr = self.inner.borrow().addr;
        let slot = slot_insn as usize;

        let length = self.inner.borrow().insn_length;

        let mut addr = unsafe { core::slice::from_raw_parts_mut(addr as *mut u8, length) };
        let mut slot_ptr = unsafe { core::slice::from_raw_parts_mut(slot as *mut u8, length) };

        addr.copy_from_slice(slot_ptr);
    }

    fn kprobes_trap_handler(&self, cx: &mut TrapFrame) {
        let mut kprobes = self.inner.borrow_mut();
        if cx.sepc == kprobes.addr {
            (kprobes.pre_handler)();
            kprobes.insn_back = cx.general.ra;
            cx.general.ra = __ebreak as usize;
            kprobes.insn_s1 = cx.general.s1;
            cx.general.s1 = cx.sepc + kprobes.insn_length;
            cx.sepc = kprobes.slot_addr;
        } else {
            cx.sepc = kprobes.insn_back;
            cx.general.s1 = kprobes.insn_s1;
            (kprobes.post_handler)();
            drop(kprobes);
        }
    }
}

pub fn kprobes_trap_handler(cx: &mut TrapFrame) {
    KPROBES.kprobes_trap_handler(cx);
}

fn slot_insn() {
    println!("???");
}
