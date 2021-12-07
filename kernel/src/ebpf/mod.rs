pub mod ebpf;
pub mod helper;
use alloc::vec::Vec;

pub fn ebpf_register(addr: usize, prog: Vec<u64>) -> isize {
    ebpf::EBPF.register(addr, prog)
}

pub fn ebpf_unregister(addr: usize) -> isize {
    ebpf::EBPF.unregister(addr)
}
