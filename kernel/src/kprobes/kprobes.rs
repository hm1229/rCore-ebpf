use alloc::collections::btree_map::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::cell::RefCell;
use core::convert::TryInto;
use core::ops::FnMut;
use core::slice::{from_raw_parts, from_raw_parts_mut};
use spin::Mutex;
use lazy_static::*;
use trapframe::TrapFrame;
use super::probes::{get_sp, ProbeType};
use riscv_insn_decode::{insn_decode, InsnStatus, get_insn_length};

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
    pub slot: [u8; 6],
    pub addisp: usize,
    pub func_ra: Vec<usize>,
    pub func_ebreak_addr: usize,
    pub insn_ebreak_addr: usize,
    pub handler: Arc<Mutex<dyn FnMut(&mut TrapFrame) + Send>>,
    pub post_handler: Option<Arc<Mutex<dyn FnMut(&mut TrapFrame) + Send>>>,
    pub probe_type: ProbeType,
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
    pub fn new(
        addr: usize,
        handler: Arc<Mutex<dyn FnMut(&mut TrapFrame) + Send>>,
        post_handler: Option<Arc<Mutex<dyn FnMut(&mut TrapFrame) + Send>>>,
        probe_type: ProbeType
    ) -> Option<Self> {
        let inst = unsafe { from_raw_parts(addr as *const u8, 4) };

        // read the lowest byte of the probed instruction to determine whether it is compressed
        let length = get_insn_length(addr);

        // save the probed instruction to a buffer
        let mut slot = [0; 6];
        slot[..length].copy_from_slice(&inst[..length]);

        // decode the probed instruction to retrive imm
        let mut addisp: usize = 0;
        let mut func_ebreak_addr:usize = 0;
        let mut insn_ebreak_addr:usize = 0;
        let ebreak = unsafe { from_raw_parts(__ebreak as *const u8, 2) };

        match probe_type{
            ProbeType::Insn =>{
                match insn_decode(addr){
                    InsnStatus::Legal =>{
                        slot[length..length+2].copy_from_slice(ebreak);
                        let length = get_insn_length(addr);
                        insn_ebreak_addr = slot.as_ptr() as usize + length;
                    },
                    _ => {warn!("kprobes: instruction is not legal"); return None},
                }
            }
            ProbeType::SyncFunc =>{
                let mut current_ebreak = CURRENT_EBREAK.inner.borrow_mut();
                let len = current_ebreak.len();
                current_ebreak.push(1);
                func_ebreak_addr = current_ebreak.as_ptr() as usize + 2 * len;
                let mut ebreak_ptr = unsafe { from_raw_parts_mut(func_ebreak_addr as *mut u8, 2)};
                ebreak_ptr.copy_from_slice(ebreak);
                match get_sp(addr){
                    Some(sp) => addisp = sp as usize,
                    None => {error!("sp not found!"); return None}
                }
                // println!("addisp {}", addisp as isize);
            }
            ProbeType::AsyncFunc =>{
                error!("not implemented yet!");
                return None
            }
        }
        Some(Self {
            addr,
            length,
            slot,
            addisp,
            func_ra: Vec::new(),
            func_ebreak_addr,
            insn_ebreak_addr,
            handler,
            post_handler,
            probe_type,
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
    fn new() -> Self {
        Self {
            inner: RefCell::new(BTreeMap::new()),
        }
    }


    fn register_kprobe(
        &self,
        addr: usize,
        handler: Arc<Mutex<dyn FnMut(&mut TrapFrame) + Send>>,
        post_handler: Option<Arc<Mutex<dyn FnMut(&mut TrapFrame) + Send>>>,
        probe_type: ProbeType,
    ) -> isize {
        let probe = KprobesInner::new(addr, handler, post_handler, probe_type);
        if let Some(probe) = probe {
            probe.arm();
            if let Some(replaced) = self.inner.borrow_mut().insert(addr, probe) {
                replaced.disarm();
            }
            info!("kprobes: register success");
            0
        } else {
            error!("kprobes: probe initialization failed");
            -1
        }
    }


    fn unregister_kprobe(&self, addr: usize) -> isize {
        if let Some(probe) = self.inner.borrow_mut().remove(&addr) {
            probe.disarm();
            return 0;
        }
        -1
    }


    fn kprobes_trap_handler(&self, cx: &mut TrapFrame) {
        let mut kprobes = self.inner.borrow_mut();
        let mut current_kprobes = CURRENT_KPROBES.inner.borrow_mut();
        match kprobes.get_mut(&cx.sepc) {
            Some(probe) => {
                // run user defined handler
                (probe.handler.lock())(cx);
                // single step the probed instruction
                match probe.probe_type{
                    ProbeType::SyncFunc =>{
                        cx.general.sp = cx.general.sp.wrapping_add(probe.addisp);
                        cx.sepc = cx.sepc.wrapping_add(probe.length);
                        if let Some(_) = probe.post_handler{
                            if !current_kprobes.contains_key(&probe.func_ebreak_addr){
                                current_kprobes.insert(probe.func_ebreak_addr, probe.clone());
                            }
                            let current_kprobe = current_kprobes.get_mut(&probe.func_ebreak_addr).unwrap();
                            current_kprobe.func_ra.push(cx.general.ra);
                            cx.general.ra = probe.func_ebreak_addr as usize;
                        }
                    },
                    ProbeType::Insn =>{
                        cx.sepc = probe.slot.as_ptr() as usize;
                        probe.insn_ebreak_addr = cx.sepc + probe.length;
                        if !current_kprobes.contains_key(&probe.insn_ebreak_addr){
                            current_kprobes.insert(probe.insn_ebreak_addr, probe.clone());
                        }
                    }
                    ProbeType::AsyncFunc => {
                        unimplemented!("probing async function is not implemented yet")
                    }
                }
            }
            None => {
                match current_kprobes.get_mut(&cx.sepc){
                    Some(probe) =>{
                        if probe.insn_ebreak_addr == cx.sepc{
                            if let Some(post_handler) = &probe.post_handler{
                                (post_handler.lock())(cx);
                            }
                            let sepc = probe.addr + probe.length;
                            current_kprobes.remove(&cx.sepc);
                            cx.sepc = sepc;
                        }
                        else{
                            (probe.post_handler.as_ref().unwrap().lock())(cx);
                            let sepc= probe.func_ra.pop().unwrap();
                            if probe.func_ra.len() == 0{
                                current_kprobes.remove(&cx.sepc);
                            }
                            cx.sepc = sepc;
                        }
                    }
                    _ => {}
                }
            }
        }
        // cx.sepc = cx.general.ra;
    }
}

pub fn kprobes_trap_handler(cx: &mut TrapFrame) {
    KPROBES.kprobes_trap_handler(cx);
}

pub fn kprobe_register(
    addr: usize, 
    handler: Arc<Mutex<dyn FnMut(&mut TrapFrame) + Send>>, 
    post_handler: Option<Arc<Mutex<dyn FnMut(&mut TrapFrame) + Send>>>, 
    probe_type: ProbeType
) -> isize {
    KPROBES.register_kprobe(addr, handler, post_handler, probe_type)
}

pub fn kprobe_unregister(addr: usize) -> isize{
    KPROBES.unregister_kprobe(addr)
}