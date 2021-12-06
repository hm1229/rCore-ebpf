mod kprobes;

pub use kprobes::kprobes_trap_handler;

pub fn kprobe_register() {
    kprobes::KPROBES.register_kprobe(crate::syscall::hook_point as usize, |cx| {
        debug!("enter kprobes")
    });
}

pub fn kprobe_unregister() {
    kprobes::KPROBES.unregister_kprobe(crate::syscall::hook_point as usize);
}
