mod kprobes;

pub use kprobes::kprobes_trap_handler;

pub fn kprobe_register() {
    kprobes::KPROBES.register_kprobe(|| debug!("enter kprobes"));
}

pub fn kprobe_unregister() {
    kprobes::KPROBES.unregister_kprobe();
}
