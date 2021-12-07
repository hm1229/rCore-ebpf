use crate::syscall;
use alloc::boxed::Box;
use alloc::collections::btree_map::BTreeMap;
use alloc::sync::Arc;
use core::borrow::BorrowMut;
use core::cell::RefCell;
use core::ops::FnMut;
use core::pin::Pin;
use core::slice::{from_raw_parts, from_raw_parts_mut};
use lazy_static::*;
use trapframe::TrapFrame;

fn sext(x: isize, size: usize) -> isize {
    let shift = core::mem::size_of::<isize>() * 8 - size;
    (x << shift) >> shift
}

pub struct Kprobes {
    pub inner: RefCell<BTreeMap<usize, KprobesInner>>,
}

pub struct KprobesInner {
    pub addr: usize,
    pub length: usize,
    pub slot: [u8; 4],
    pub handler: Box<dyn FnMut(&mut TrapFrame) + Send>,
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
    pub fn new(addr: usize, handler: Box<dyn FnMut(&mut TrapFrame) + Send>) -> Self {
        let inst = unsafe { from_raw_parts(addr as *const u8, 4) };
        // read the lowest byte of the probed instruction to determine whether it is compressed
        let length = if inst[0] & 0b11 == 0b11 { 4 } else { 2 };
        // save the probed instruction to a buffer
        let mut slot = [0; 4];
        slot[..length].copy_from_slice(&inst[..length]);
        Self {
            addr,
            length,
            slot,
            handler,
        }
    }
    pub fn arm(&self) {
        let ebreak = unsafe { from_raw_parts(__ebreak as *const u8, self.length) };
        let mut inst = unsafe { from_raw_parts_mut(self.addr as *mut u8, self.length) };
        inst.copy_from_slice(ebreak);
        unsafe { asm!("fence.i") };
    }
    pub fn disarm(&self) {
        let mut inst = unsafe { from_raw_parts_mut(self.addr as *mut u8, self.length) };
        inst.copy_from_slice(&self.slot[..self.length]);
        unsafe { asm!("fence.i") };
    }
}

impl Kprobes {
    pub fn new() -> Self {
        Self {
            inner: RefCell::new(BTreeMap::new()),
        }
    }
    pub fn register_kprobe(
        &self,
        addr: usize,
        handler: Box<dyn FnMut(&mut TrapFrame) + Send>,
    ) -> isize {
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
        match kprobes.get_mut(&cx.sepc) {
            Some(probe) => {
                // run user defined handler
                (probe.handler)(cx);
                // single step the probed instruction
                match probe.length {
                    4 => {
                        // normal instruction
                        let mut buf: [u8; 4] = [0; 4];
                        buf.copy_from_slice(&probe.slot[..4]);
                        let inst = u32::from_le_bytes(buf);
                        if inst & 0b00000000000000000000000000010011
                            == 0b00000000000000000000000000010011
                        {
                            // addi
                            let rd: u8 = ((inst >> 7) & 0b11111) as u8;
                            let rs1: u8 = ((inst >> 15) & 0b11111) as u8;
                            let imm: isize = sext(((inst >> 20) & 0b111111111111) as isize, 12);
                            if rd != 2 || rs1 != 2 {
                                panic!("rs1 or rd not sp");
                            }
                            cx.general.sp += imm as usize;
                        } else {
                            panic!("not addi");
                        }
                    }
                    _ => {
                        // compressed instruction
                        let mut buf: [u8; 2] = [0; 2];
                        buf.copy_from_slice(&probe.slot[..2]);
                        let inst = u16::from_le_bytes(buf);
                        if inst & 0b0000000000000001 == 0b0000000000000001 {
                            // c.addi
                            let rd: u8 = (inst >> 7 & 0b11111) as u8;
                            let imm: isize = ((((inst >> 12) & 0b1) << 5)
                                + (((inst >> 2) & 0b11111) << 0))
                                as isize;
                            let imm: isize = sext(imm, 6);
                            if rd != 2 {
                                panic!("rs1/rd not sp");
                            }
                            cx.general.sp += imm as usize;
                        } else {
                            panic!("not c.addi");
                        }
                    }
                };
                cx.sepc += probe.length;
            }
            None => {}
        }
    }
}

pub fn kprobes_trap_handler(cx: &mut TrapFrame) {
    KPROBES.kprobes_trap_handler(cx);
}
