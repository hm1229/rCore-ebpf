use alloc::boxed::Box;
use alloc::collections::btree_map::BTreeMap;
use alloc::string::String;
use core::cell::RefCell;
use spin::Mutex;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::convert::TryInto;
use core::slice::{from_raw_parts, from_raw_parts_mut};
use riscv_insn_decode::get_insn_length;
use trapframe::UserContext;
use super::kprobes::kprobe_register;
use super::uprobes::uprobe_register;

pub fn get_sp(addr: usize) -> Option<usize>{
    let slot = unsafe { from_raw_parts(addr as *const u8, 4) };
    let mut addisp: usize= 0;
    match get_insn_length(addr) {
        4 => {
            // normal instruction
            let inst = u32::from_le_bytes(slot[..4].try_into().unwrap());
            if inst & 0b00000000000011111111111111111111 == 0b00000000000000010000000100010011 {
                // addi sp, sp, imm
                addisp = sext(((inst >> 20) & 0b111111111111) as isize, 12) as usize;
                debug!("kprobes: hook on addi sp, sp, {}", addisp);
            } else {
                warn!("kprobes: target instruction is not addi sp, sp, imm");
                return None;
            }
        }
        2 => {
            // compressed instruction
            let inst = u16::from_le_bytes(slot[..2].try_into().unwrap());
            if inst & 0b1110111110000011 == 0b0110000100000001 {
                // c.addi16sp imm
                addisp = sext(
                    ((((inst >> 12) & 0b1) << 9)
                        + (((inst >> 6) & 0b1) << 4)
                        + (((inst >> 5) & 0b1) << 6)
                        + (((inst >> 3) & 0b11) << 7)
                        + (((inst >> 2) & 0b1) << 5)) as isize,
                    10,
                ) as usize;
                debug!("kprobes: hook on c.addi16sp {}", addisp as isize);
            } else if inst & 0b1110111110000011 == 0b0000000100000001 {
                // c.addi sp, imm
                addisp = sext(
                    ((((inst >> 12) & 0b1) << 5) + (((inst >> 2) & 0b11111) << 0)) as isize,
                    6,
                ) as usize;
                debug!("kprobes: hook on c.addi sp, {}", addisp as isize);
            } else if  inst & 0b1110000000000011 == 0 {
                // c.addi4spn
                addisp = sext(((((inst >> 11) & 0b111) << 3)
                    + (((inst >> 7) & 0b1111) << 5)
                    + (((inst >> 6) & 0b1) << 1)
                    + (((inst >> 5) & 0b1) << 2)) as isize,
                    10
                ) as usize;
                // println!("kprobes: hook on c.addi4spn, {}", addisp);
            } else {
                error!("kprobes: target instruction is not c.addi sp, imm or c.addi16sp imm or c.addi4spn imm");
                return None;
            }
        }
        _ => return None
    };
    Some(addisp)
}

fn sext(x: isize, size: usize) -> isize {
    let shift = core::mem::size_of::<isize>() * 8 - size;
    (x << shift) >> shift
}

#[derive(Clone, Debug)]
pub enum ProbePlace {
    Kernel(ProbeType),
    User(ProbeType),
}

#[derive(Clone, Debug)]
pub enum ProbeType {
    Insn,
    SyncFunc,
    AsyncFunc,
}