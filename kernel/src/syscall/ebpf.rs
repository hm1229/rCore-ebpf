use super::*;
use core::convert::TryInto;
use crate::kprobes::ProbePlace;
use core::mem::transmute;

impl Syscall<'_> {
    pub fn sys_register_ebpf(&mut self, addr: usize, base: *const u8, len: usize, pt: usize, path: *const u8) -> SysResult {
        let slice = unsafe { self.vm().check_read_array(base, len)? };
        let path = check_and_clone_cstr(path)?;
        let pp: ProbePlace = unsafe { transmute(pt as u16)};
        let prog = slice
            .chunks_exact(8)
            .map(|x| u64::from_le_bytes(x.try_into().unwrap()))
            .collect::<alloc::vec::Vec<u64>>();
        // println!("path");
        if crate::ebpf::ebpf_register(addr, prog, path, pp) != 0 {
            return Err(SysError::EINVAL);
        }
        Ok(0)
    }

    pub fn sys_unregister_ebpf(&mut self, addr: usize) -> SysResult {
        if crate::ebpf::ebpf_unregister(addr) != 0 {
            return Err(SysError::EINVAL);
        }
        Ok(0)
    }

    pub async fn sys_test_async(&mut self) -> SysResult {
        Ok(0)
    }
}