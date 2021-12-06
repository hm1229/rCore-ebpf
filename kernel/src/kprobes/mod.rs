mod kprobes;
use alloc::boxed::Box;
pub use kprobes::kprobes_trap_handler;
use trapframe::TrapFrame;

pub fn kprobe_register(addr: usize, handler: Box<dyn FnMut(&mut TrapFrame) + Send>) {
    kprobes::KPROBES.register_kprobe(addr, handler);
}

pub fn kprobe_unregister(addr: usize) {
    kprobes::KPROBES.unregister_kprobe(addr);
}
