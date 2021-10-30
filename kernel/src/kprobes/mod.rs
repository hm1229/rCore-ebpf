mod kprobes;
mod kprobe_pre_handler;
mod kprobe_post_handler;

pub use kprobes::{KPROBES, KprobesStatus, kprobes_trap_handler};
use kprobe_pre_handler::pre_handler;
use kprobe_post_handler::post_handler;

pub fn kprobe_register(){
    KPROBES.register_kprobe(pre_handler, post_handler);
}