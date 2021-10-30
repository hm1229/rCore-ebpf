use alloc::string::String;
use lazy_static::*;
use crate::syscall;
use trapframe::TrapFrame;
use core::cell::RefCell;

global_asm!(include_str!("ebreak.S"));

pub struct Kprobes{
    pub inner: RefCell<KprobesInner>,
}

pub enum KprobesStatus {
    HIT,
    PREPARE,
}

pub struct KprobesInner{
    /* 在被探测点指令执行之前调用的回调函数 */
    pub pre_handler : fn(),

    /* 在被探测点指令执行之后调用的回调函数 */
    pub post_handler : fn(),
    
    /* 被复制的被探测点的原始指令（Risc-V） */
    pub insn_back : usize,

    /* 状态标记 */
    // flags : u32,

    pub kprobe_status : KprobesStatus,

    pub breakpoint_addr: usize,
}
unsafe impl Sync for Kprobes {}

lazy_static!{
    pub static ref KPROBES: Kprobes = Kprobes::new();
}


extern "C" { pub fn __ebreak(); }

impl Kprobes{
    /* 向内核注册kprobe探测点 */
    fn new() -> Self{
        Self{
            inner: RefCell::new(KprobesInner{
                pre_handler: test,
                post_handler: test,
                insn_back: 0,
                kprobe_status: KprobesStatus::PREPARE,
                breakpoint_addr: __ebreak as usize,
            })
        }
    }
    pub fn register_kprobe(&self, pre_handler: fn(), post_handler: fn()){
        let mut inner = self.inner.borrow_mut();
        inner.pre_handler = pre_handler;
        inner.post_handler = post_handler;
        inner.kprobe_status = KprobesStatus::PREPARE;
        drop(inner);
        self.prepare_kprobe();
    }
    pub fn prepare_kprobe(&self){
        let addr = syscall::handle_syscall as usize;
        let addr_break = __ebreak as usize;
        let slot = slot_insn as usize;

        let mut addr = unsafe{core::slice::from_raw_parts_mut(addr as *mut u8, 2)};
        let mut addr_break = unsafe{core::slice::from_raw_parts_mut(addr_break as *mut u8, 2)};
        let mut slot_ptr = unsafe{core::slice::from_raw_parts_mut(slot as *mut u8, 2)};

        slot_ptr.copy_from_slice(addr);
        addr.copy_from_slice(addr_break);
    }

    pub fn restore_kprobe(&self){
        let addr = syscall::handle_syscall as usize;
        let slot = slot_insn as usize;

        let mut addr = unsafe{core::slice::from_raw_parts_mut(addr as *mut u8, 2)};
        let mut slot_ptr = unsafe{core::slice::from_raw_parts_mut(slot as *mut u8, 2)};

        addr.copy_from_slice(slot_ptr);
    }



    /* 卸载kprobe探测点 */
    fn unregister_kprobe(&self) -> usize{
        unimplemented!()
    }
    
    fn kprobes_trap_handler(&self, cx: &mut TrapFrame){
        let mut kprobes = self.inner.borrow_mut();
        match kprobes.kprobe_status {
            KprobesStatus::HIT => {
                kprobes.kprobe_status = KprobesStatus::PREPARE;
                cx.sepc = kprobes.insn_back;
                (kprobes.post_handler)();
                drop(kprobes);
                KPROBES.prepare_kprobe();
            }
            KprobesStatus::PREPARE =>{
                (kprobes.pre_handler)();
                kprobes.insn_back = cx.general.ra;
                cx.general.ra = kprobes.breakpoint_addr;
                kprobes.kprobe_status = KprobesStatus::HIT;
                drop(kprobes);
                KPROBES.restore_kprobe();
            }
            _ => {
                panic!("wrong status");
            }
        }
    }
}

pub fn kprobes_trap_handler(cx: &mut TrapFrame){
    KPROBES.kprobes_trap_handler(cx);
}

fn test(){
    println!("use!");
}

fn slot_insn(){
    println!("???");
}
