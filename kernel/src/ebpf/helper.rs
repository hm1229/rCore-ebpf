use ebpf_rs::interpret::Helper;
use crate::process::current_thread;

pub const HELPERS: [Helper; 16] = [
    nop,
    nop,
    nop,
    nop,
    nop,
    bpf_ktime_get_ns,
    bpf_trace_printk,
    nop,
    nop,
    nop,
    nop,
    nop,
    nop,
    bpf_get_current_pid_tgid,
    nop,
    nop,
];

pub fn nop(_: u64, _: u64, _: u64, _: u64, _: u64) -> u64 {
    0
}

// long bpf_trace_printk(const char *fmt, u32 fmt_size, ...)
unsafe fn bpf_trace_printk(fmt: u64, fmt_size: u64, p1: u64, p2: u64, p3: u64) -> u64 {
    let fmt = core::slice::from_raw_parts(fmt as *const u8, fmt_size as u32 as usize);
    println!(
        "{}",
        dyn_fmt::Arguments::new(core::str::from_utf8_unchecked(fmt), &[format!("{:#x}", p1),
            format!("{}", p2), format!("{}", p3)])
    );
    0
}

// u64 bpf_ktime_get_ns(void)
// return current ktime
fn bpf_ktime_get_ns(_1: u64, _2: u64, _3: u64, _4: u64, _5: u64) -> u64 {
    crate::arch::timer::timer_now().as_nanos() as u64
}

fn bpf_get_current_pid_tgid(_1: u64, _2: u64, _3: u64, _4: u64, _5: u64) -> u64 {
    let thread = current_thread().unwrap();
    let pid = thread.proc.busy_lock().pid.get() as u64;
    // NOTE: tgid is the same with pid
    (pid << 32) | pid
}
