pub mod ebpf;
pub mod helper;

use alloc::string::String;
use alloc::vec::Vec;
pub use ebpf::test_async;

pub fn ebpf_register(addr: usize, prog: Vec<u64>, path: String) -> isize {
    ebpf::EBPF.register(addr, prog, path)
}

pub fn ebpf_unregister(addr: usize) -> isize {
    ebpf::EBPF.unregister(addr)
}

