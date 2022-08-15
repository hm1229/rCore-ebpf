mod probes;
mod kprobes;
mod uprobes;
// mod riscv_insn_decode;

use alloc::sync::Arc;
pub use kprobes::{kprobes_trap_handler, kprobe_register, kprobe_unregister};
pub use uprobes::{uprobes_trap_handler, uprobe_register, uprobes_init};
pub use probes::{ProbePlace, ProbeType};


