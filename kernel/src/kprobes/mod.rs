mod kprobes;
use alloc::sync::Arc;
pub use kprobes::kprobes_trap_handler;
use spin::Mutex;
use trapframe::TrapFrame;

pub fn kprobe_register(addr: usize, handler: Arc<Mutex<dyn FnMut(&mut TrapFrame) + Send>>, post_handler: Option<Arc<Mutex<dyn FnMut(&mut TrapFrame) + Send>>>) -> isize {
    kprobes::KPROBES.register_kprobe(addr, handler, post_handler)
}

pub fn kprobe_unregister(addr: usize) -> isize {
    kprobes::KPROBES.unregister_kprobe(addr)
}

