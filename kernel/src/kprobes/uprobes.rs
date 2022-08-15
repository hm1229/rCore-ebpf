use alloc::collections::btree_map::BTreeMap;
use alloc::string::String;
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
use rcore_memory::memory_set::MemoryAttr;
use rcore_memory::memory_set::handler::{Delay, ByFrame};
use rcore_memory::paging::PageTable;
use riscv_insn_decode::{insn_decode, InsnStatus, get_insn_length};
use super::probes::{get_sp, ProbeType};
use crate::memory::{AccessType, handle_page_fault_ext, GlobalFrameAlloc};
use crate::process::current_thread;
use trapframe::UserContext;


pub struct Uprobes {
    pub inner: RefCell<BTreeMap<usize, UprobesInner>>,
}

struct CurrentUprobes{
    inner: RefCell<BTreeMap<usize, UprobesInner>> ,
}

struct CurrentProcessUprobesInner{
    uprobes: Uprobes,
    current_uprobes: CurrentUprobes,
}

struct CurrentProcessUprobes{
    inner: RefCell<BTreeMap<String, CurrentProcessUprobesInner>>,
}

#[derive(Clone)]
pub struct UprobesInner {
    pub addr: usize,
    pub length: usize,
    pub slot_addr: usize,
    pub addisp: usize,
    pub func_ra: Vec<usize>,
    pub func_ebreak_addr: usize,
    pub insn_ebreak_addr: usize,
    pub handler: Arc<Mutex<dyn FnMut(&mut UserContext) + Send>>,
    pub post_handler: Option<Arc<Mutex<dyn FnMut(&mut UserContext) + Send>>>,
    pub probe_type: ProbeType,
}


unsafe impl Sync for Uprobes {}
unsafe impl Sync for UprobesInner {}
unsafe impl Sync for CurrentUprobes {}
unsafe impl Sync for CurrentProcessUprobes {}
unsafe impl Sync for CurrentProcessUprobesInner {}

lazy_static! {
    pub static ref UPROBES: Uprobes = Uprobes::new();
}

lazy_static! {
    static ref CURRENT_PROCESS_UPROBES: CurrentProcessUprobes = CurrentProcessUprobes::new();
}

#[naked]
extern "C" fn __ebreak() {
    unsafe {
        asm!("c.ebreak", "c.ebreak");
    }
}

impl CurrentProcessUprobes{
    fn new() -> Self{
        Self{
            inner: RefCell::new(BTreeMap::new()),
        }
    }

    fn uprobes_init(&self){
        let path = get_exec_path();
        if let Some(inner) = self.inner.borrow().get(&path){
            inner.uprobes.add_uprobepoint();
        }
    }

    fn register_uprobes(
        &self,
        path: String,
        addr: usize,
        handler: Arc<Mutex<dyn FnMut(&mut UserContext) + Send>>,
        post_handler: Option<Arc<Mutex<dyn FnMut(&mut UserContext) + Send>>>,
        probe_type: ProbeType
    ) -> isize {
        let mut uprobes_inner = self.inner.borrow_mut();
        if let Some(inner) = uprobes_inner.get_mut(&path.clone()){
            inner.uprobes.register_uprobe(addr, handler, post_handler, probe_type);
        }
        else{
            let uprobes = Uprobes::new();
            info!("uprobes: add new path");
            uprobes.register_uprobe(addr, handler, post_handler, probe_type);
            let current_uprobes = CurrentUprobes::new();
            uprobes_inner.insert(path.clone(), CurrentProcessUprobesInner{
                uprobes,
                current_uprobes,
            });
            info!("uprobes: insert success");
        }
        info!("uprobes: path={}", get_exec_path());
        if path == get_exec_path(){
            info!("uprobes: path=execpath");
            uprobes_inner.get_mut(&path.clone()).unwrap().uprobes.inner.borrow_mut().get_mut(&addr).unwrap().add_uprobepoint();
            info!("uprobes: path=execpath, add sucess");
        }
        0
    }

    fn uprobes_trap_handler(&self, cx: &mut UserContext){
        let path = get_exec_path();
        let mut uprobes_inner = self.inner.borrow_mut();
        let mut uprobes = uprobes_inner.get(&path.clone()).unwrap().uprobes.inner.borrow_mut();
        let mut current_uprobes = uprobes_inner.get(&path.clone()).unwrap().current_uprobes.inner.borrow_mut();
        match uprobes.get_mut(&cx.sepc) {
            Some(probe) => {
                // run user defined handler
                (probe.handler.lock())(cx);
                // single step the probed instruction
                match probe.probe_type{
                    ProbeType::SyncFunc =>{
                        cx.general.sp = cx.general.sp.wrapping_add(probe.addisp);
                        cx.sepc = cx.sepc.wrapping_add(probe.length);
                        if let Some(_) = probe.post_handler{
                            if !current_uprobes.contains_key(&probe.func_ebreak_addr){
                                current_uprobes.insert(probe.func_ebreak_addr, probe.clone());
                            }
                            let current_uprobe = current_uprobes.get_mut(&probe.func_ebreak_addr).unwrap();
                            current_uprobe.func_ra.push(cx.general.ra);
                            cx.general.ra = probe.func_ebreak_addr as usize;
                        }
                    },
                    ProbeType::Insn =>{
                        cx.sepc = probe.slot_addr as usize;
                        probe.insn_ebreak_addr = cx.sepc + probe.length;
                        if !current_uprobes.contains_key(&probe.insn_ebreak_addr){
                            current_uprobes.insert(probe.insn_ebreak_addr, probe.clone());
                        }
                    }
                    ProbeType::AsyncFunc => {
                        unimplemented!("probing async function is not implemented yet")
                    }
                }
            }
            None => {
                match current_uprobes.get_mut(&cx.sepc){
                    Some(probe) =>{
                        if probe.insn_ebreak_addr == cx.sepc{
                            if let Some(post_handler) = &probe.post_handler{
                                (post_handler.lock())(cx);
                            }
                            let sepc = probe.addr + probe.length;
                            current_uprobes.remove(&cx.sepc);
                            cx.sepc = sepc;
                        }
                        else{
                            (probe.post_handler.as_ref().unwrap().lock())(cx);
                            cx.sepc = probe.func_ra.pop().unwrap();
                            if probe.func_ra.len() == 0{
                                current_uprobes.remove(&cx.sepc);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

impl CurrentUprobes{
    fn new() -> Self{
        Self{
            inner: RefCell::new(BTreeMap::new()),
        }
    }
}

impl UprobesInner {
    pub fn new(
        addr: usize,
        handler: Arc<Mutex<dyn FnMut(&mut UserContext) + Send>>,
        post_handler: Option<Arc<Mutex<dyn FnMut(&mut UserContext) + Send>>>,
        probe_type: ProbeType
    ) -> Option<Self> {
        Some(Self {
            addr,
            length: 0,
            slot_addr: 0,
            addisp: 0,
            func_ra: Vec::new(),
            func_ebreak_addr: 0,
            insn_ebreak_addr: 0,
            handler,
            post_handler,
            probe_type,
        })
    }

    fn add_uprobepoint(&mut self){
        // get free point in user stack
        let addr = self.addr;
        self.func_ebreak_addr = get_new_page(addr, 2);
        self.slot_addr = get_new_page(addr, 6);
        let mut slot = unsafe { from_raw_parts_mut(self.slot_addr as *mut u8, 6)};
        set_writeable(addr);

        let inst = unsafe { from_raw_parts(addr as *const u8, 2) };
        // read the lowest byte of the probed instruction to determine whether it is compressed
        let length = get_insn_length(addr);
        self.length = length;
        // save the probed instruction to a buffer
        slot[..length].copy_from_slice(&inst[..length]);

        // decode the probed instruction to retrive imm
        let ebreak = unsafe { from_raw_parts(__ebreak as *const u8, 2) };

        match self.probe_type{
            ProbeType::Insn =>{
                match insn_decode(addr){
                    InsnStatus::Legal =>{
                        slot[length..length+2].copy_from_slice(ebreak);
                        self.insn_ebreak_addr = self.slot_addr + length;
                    },
                    _ => {warn!("uprobes: instruction is not legal");},
                }
            }
            ProbeType::SyncFunc =>{
                let mut ebreak_ptr = unsafe { from_raw_parts_mut(self.func_ebreak_addr as *mut u8, 2)};
                ebreak_ptr.copy_from_slice(ebreak);

                match get_sp(addr){
                    Some(sp) => self.addisp = sp,
                    None => {error!("sp not found!");}
                }
            }
            ProbeType::AsyncFunc =>{
                error!("not implemented yet!");
            }
        }
        self.arm()
    }

    pub fn arm(&self) {
        let ebreak = unsafe { from_raw_parts(__ebreak as *const u8, self.length) };
        let mut inst = unsafe { from_raw_parts_mut(self.addr as *mut u8, self.length) };
        inst.copy_from_slice(ebreak);
        unsafe { asm!("fence.i") };
    }

    pub fn disarm(&self) {
        let mut inst = unsafe { from_raw_parts_mut(self.addr as *mut u8, self.length) };
        let slot = unsafe { from_raw_parts(self.slot_addr as *const u8, self.length)};
        inst.copy_from_slice(slot);
        unsafe { asm!("fence.i") };
    }
}

impl Uprobes {
    fn register_uprobe(
        &self,
        addr: usize,
        handler: Arc<Mutex<dyn FnMut(&mut UserContext) + Send>>,
        post_handler: Option<Arc<Mutex<dyn FnMut(&mut UserContext) + Send>>>,
        probe_type: ProbeType,
    ) -> isize{
        let probe = UprobesInner::new(addr, handler, post_handler, probe_type);
        if let Some(probe) = probe {
            self.inner.borrow_mut().insert(addr, probe);
            info!("uprobes: register success");
            1
        } else {
            error!("uprobes: probe initialization failed");
            -1
        }
    }

    fn new() -> Self {
        Self {
            inner: RefCell::new(BTreeMap::new()),
        }
    }

    fn uprobes_trap_handler(&self, cx: &mut UserContext) {

    }

    fn add_uprobepoint(&self){
        let mut uproebs = self.inner.borrow_mut();
        for inner in uproebs.values_mut(){
            inner.add_uprobepoint();
        }
    }
}


fn get_new_page(addr: usize, len: usize) -> usize{
    let thread = current_thread().unwrap();
    let mut vm = thread.vm.lock();
    let ebreak_addr = vm.find_free_area(addr, len);
    vm.push(
        ebreak_addr,
        ebreak_addr + len,
        MemoryAttr::default().user().execute().writable(),
        ByFrame::new(GlobalFrameAlloc),
        "point",
    );
    unsafe {asm!("fence.i");}
    ebreak_addr
}

fn set_writeable(addr: usize){
    let thread = current_thread().unwrap();
    let mut vm = thread.vm.lock();
    let mut page_table_entry = vm.get_page_table_mut().get_entry(addr).unwrap();
    page_table_entry.set_writable(true);
    unsafe {asm!("fence.i");}
}

fn get_exec_path() -> String{
    info!("uprobes: get path");
    let ret = current_thread().unwrap().proc.try_lock().expect("locked!").exec_path.clone();
    info!("uprobes get path success path = {}", ret);
    ret
}

pub fn uprobe_register(
    path: String,
    addr: usize,
    handler: Arc<Mutex<dyn FnMut(&mut UserContext) + Send>>,
    post_handler: Option<Arc<Mutex<dyn FnMut(&mut UserContext) + Send>>>,
    probe_type: ProbeType
) -> isize {
    CURRENT_PROCESS_UPROBES.register_uprobes(path ,addr, handler, post_handler, probe_type)
}

pub fn uprobes_trap_handler(cx: &mut UserContext) {
    info!("uprobes: into uprobes trap handler");
    CURRENT_PROCESS_UPROBES.uprobes_trap_handler(cx);
}

pub fn uprobes_init(){
    CURRENT_PROCESS_UPROBES.uprobes_init();
    info!("uprobes: init sucess");
}