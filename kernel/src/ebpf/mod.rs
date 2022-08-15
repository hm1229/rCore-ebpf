pub mod ebpf;
pub mod helper;

use alloc::string::String;
use alloc::vec::Vec;
pub use ebpf::test_async;
use crate::kprobes::ProbePlace;

pub fn ebpf_register(addr: usize, prog: Vec<u64>, path: String, pp: ProbePlace) -> isize {
    ebpf::EBPF.register(addr, prog, path, pp)
}

pub fn ebpf_unregister(addr: usize) -> isize {
    ebpf::EBPF.unregister(addr)
}

