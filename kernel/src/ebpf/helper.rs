use ebpf_rs::interpret::Helper;

pub const HELPERS: [Helper; 16] = [
    nop,
    nop,
    nop,
    nop,
    nop,
    nop,
    bpf_trace_printk,
    nop,
    nop,
    nop,
    nop,
    nop,
    nop,
    nop,
    nop,
    nop,
];

pub fn nop(_: u64, _: u64, _: u64, _: u64, _: u64) -> u64 {
    0
}

unsafe fn bpf_trace_printk(fmt: u64, fmt_size: u64, p1: u64, p2: u64, p3: u64) -> u64 {
    let fmt = core::slice::from_raw_parts(fmt as *const u8, fmt_size as u32 as usize);
    print!(
        "{}",
        dyn_fmt::Arguments::new(core::str::from_utf8_unchecked(fmt), &[p1, p2, p3])
    );
    0
}
