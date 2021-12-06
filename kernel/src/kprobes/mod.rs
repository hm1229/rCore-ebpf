mod kprobe_post_handler;
mod kprobe_pre_handler;
mod kprobes;

use kprobe_post_handler::post_handler;
use kprobe_pre_handler::pre_handler;
pub use kprobes::{kprobes_trap_handler, KPROBES};

pub fn kprobe_register() {
    KPROBES.register_kprobe(pre_handler, post_handler);
}

pub fn kprobe_unregister() {
    KPROBES.unregister_kprobe();
}
