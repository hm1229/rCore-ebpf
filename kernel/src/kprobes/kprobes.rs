use crate::syscall;
// use alloc::boxed::Box;
use alloc::collections::btree_map::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::borrow::BorrowMut;
use core::cell::RefCell;
use core::convert::TryInto;
use core::ops::FnMut;
use core::pin::Pin;
use core::slice::{from_raw_parts, from_raw_parts_mut};
use spin::Mutex;
use lazy_static::*;
use trapframe::TrapFrame;

fn sext(x: isize, size: usize) -> isize {
    let shift = core::mem::size_of::<isize>() * 8 - size;
    (x << shift) >> shift
}

pub struct Kprobes {
    pub inner: RefCell<BTreeMap<usize, KprobesInner>>,
}

struct CurrentKprobes{
    inner: RefCell<BTreeMap<usize, KprobesInner>> ,
}

struct CurrentEbreaks{
    inner: RefCell<Vec<u16>>,
}

#[derive(Clone)]
pub struct KprobesInner {
    pub addr: usize,
    pub length: usize,
    pub slot: [u8; 4],
    pub addisp: usize,
    pub func_ra: Vec<usize>,
    pub ebreak_addr: usize,
    pub handler: Arc<Mutex<dyn FnMut(&mut TrapFrame) + Send>>,
    pub post_handler: Option<Arc<Mutex<dyn FnMut(&mut TrapFrame) + Send>>>,
}


unsafe impl Sync for Kprobes {}
unsafe impl Sync for KprobesInner {}
unsafe impl Sync for CurrentEbreaks {}
unsafe impl Sync for CurrentKprobes {}

lazy_static! {
    pub static ref KPROBES: Kprobes = Kprobes::new();
}

lazy_static! {
    static ref CURRENT_EBREAK: CurrentEbreaks= CurrentEbreaks::new();
}

lazy_static! {
    static ref CURRENT_KPROBES: CurrentKprobes = CurrentKprobes::new();
}

#[naked]
extern "C" fn __ebreak() {
    unsafe {
        asm!("c.ebreak", "c.ebreak");
    }
}

impl CurrentEbreaks{
    fn new() -> Self{
        Self{
            inner: RefCell::new(Vec::new()),
        }
    }
}

impl CurrentKprobes{
    fn new() -> Self{
        Self{
            inner: RefCell::new(BTreeMap::new()),
        }
    }
}

impl KprobesInner {
    pub fn new(addr: usize, handler: Arc<Mutex<dyn FnMut(&mut TrapFrame) + Send>>, post_handler: Option<Arc<Mutex<dyn FnMut(&mut TrapFrame) + Send>>>) -> Option<Self> {
        let inst = unsafe { from_raw_parts(addr as *const u8, 4) };
        // read the lowest byte of the probed instruction to determine whether it is compressed
        let length = if inst[0] & 0b11 == 0b11 { 4 } else { 2 };
        // save the probed instruction to a buffer
        let mut slot = [0; 4];
        slot[..length].copy_from_slice(&inst[..length]);
        // decode the probed instruction to retrive imm
        let mut addisp: usize = 0;
        let mut current_ebreak = CURRENT_EBREAK.inner.borrow_mut();
        let len = current_ebreak.len();
        current_ebreak.push(1);
        let ebreak_addr = current_ebreak.as_ptr() as usize + 2 * len;
        let ebreak = unsafe { from_raw_parts(__ebreak as *const u8, 2) };
        let mut ebreak_ptr = unsafe { from_raw_parts_mut(ebreak_addr as *mut u8, 2)};
        ebreak_ptr.copy_from_slice(ebreak);
        match length {
            4 => {
                // normal instruction
                let inst = u32::from_le_bytes(slot[..length].try_into().unwrap());
                if inst & 0b00000000000011111111111111111111 == 0b00000000000000010000000100010011 {
                    // addi sp, sp, imm
                    addisp = sext(((inst >> 20) & 0b111111111111) as isize, 12) as usize;
                    debug!("kprobes: hook on addi sp, sp, {}", addisp as isize);
                } else {
                    warn!("kprobes: target instruction is not addi sp, sp, imm");
                    return None;
                }
            }
            2 => {
                // compressed instruction
                let inst = u16::from_le_bytes(slot[..length].try_into().unwrap());
                if inst & 0b1110111110000011 == 0b0110000100000001 {
                    // c.addi16sp imm
                    addisp = sext(
                        ((((inst >> 12) & 0b1) << 9)
                            + (((inst >> 6) & 0b1) << 4)
                            + (((inst >> 5) & 0b1) << 6)
                            + (((inst >> 3) & 0b11) << 7)
                            + (((inst >> 2) & 0b1) << 5)) as isize,
                        10,
                    ) as usize;
                    debug!("kprobes: hook on c.addi16sp {}", addisp as isize);
                } else if inst & 0b1110111110000011 == 0b0000000100000001 {
                    // c.addi sp, imm
                    addisp = sext(
                        ((((inst >> 12) & 0b1) << 5) + (((inst >> 2) & 0b11111) << 0)) as isize,
                        6,
                    ) as usize;
                    debug!("kprobes: hook on c.addi sp, {}", addisp as isize);
                } else {
                    warn!("kprobes: target instruction is not c.addi sp, imm or c.addi16sp imm");
                    return None;
                }
            }
            _ => return None,
        };
        Some(Self {
            addr,
            length,
            slot,
            addisp,
            func_ra: Vec::new(),
            ebreak_addr,
            handler,
            post_handler,
        })
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
        handler: Arc<Mutex<dyn FnMut(&mut TrapFrame) + Send>>,
        post_handler: Option<Arc<Mutex<dyn FnMut(&mut TrapFrame) + Send>>>,
    ) -> isize {
        let probe = KprobesInner::new(addr, handler, post_handler);
        if let Some(probe) = probe {
            probe.arm();
            if let Some(replaced) = self.inner.borrow_mut().insert(addr, probe) {
                replaced.disarm();
            }
            0
        } else {
            error!("kprobes: probe initialization failed");
            -1
        }
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
        let mut current_kprobes = CURRENT_KPROBES.inner.borrow_mut();
        match kprobes.get_mut(&cx.sepc) {
            Some(probe) => {
                // run user defined handler
                (probe.handler.lock())(cx);
                // single step the probed instruction
                cx.general.sp = cx.general.sp.wrapping_add(probe.addisp);
                cx.sepc = cx.sepc.wrapping_add(probe.length);
                if let Some(_) = probe.post_handler{
                    if !current_kprobes.contains_key(&probe.ebreak_addr){
                        current_kprobes.insert(probe.ebreak_addr, probe.clone());
                    }
                    let current_kprobe = current_kprobes.get_mut(&probe.ebreak_addr).unwrap();
                    current_kprobe.func_ra.push(cx.general.ra);
                    cx.general.ra = probe.ebreak_addr as usize;
                }
            }
            None => {
                match current_kprobes.get_mut(&cx.sepc){
                    Some(probe) =>{
                        (probe.post_handler.as_ref().unwrap().lock())(cx);
                        cx.sepc = probe.func_ra.pop().unwrap();
                        if probe.func_ra.len() == 0{
                            current_kprobes.remove(&cx.sepc);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

pub fn kprobes_trap_handler(cx: &mut TrapFrame) {
    KPROBES.kprobes_trap_handler(cx);
}

