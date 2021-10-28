static long (*bpf_trace_printk)(const char* fmt, int fmt_size, long p1, long p2, long p3) = (void*)6;

typedef struct TrapFrame {
    long rax;
    long rbx;
    long rcx;
    long rdx;
    long rsi;
    long rdi;
    long rbp;
    long rsp;
    long r8;
    long r9;
    long r10;
    long r11;
    long r12;
    long r13;
    long r14;
    long r15;
    long _pad;
    long trap_num;
    long error_code;
    long rip;
    long cs;
    long rflags;
} TrapFrame;

typedef struct SyscallContext {
    long num;
    long args[6];
} SyscallContext;

long syscall(struct TrapFrame* tf)
{
    char name[] = "read writeopen closestat ";
    SyscallContext* ctx = (void*)tf->r10;
    if (ctx->num < 5) {
        bpf_trace_printk(name + 5 * ctx->num, 5, 0, 0, 0);
    }
    return 0;
}
