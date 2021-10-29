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
    char read[] = "read ";
    char write[] = "write";
    char open[] = "open ";
    char close[] = "close";
    char stat[] = "stat ";
    char fmt1[] = "enter syscall name:      ";
    char fmt2[] = "enter syscall number: {}";
    SyscallContext* ctx = (void*)tf->r10;
    if (ctx->num < 5) {
        char* buf;
        switch (ctx->num) {
        case 0:
            buf = read;
            break;
        case 1:
            buf = write;
            break;
        case 2:
            buf = open;
            break;
        case 3:
            buf = close;
            break;
        case 4:
            buf = stat;
            break;
        }
        for (int i = 0; i < 5; i++)
            fmt1[20 + i] = buf[i];
        bpf_trace_printk(fmt1, sizeof(fmt1), 0, 0, 0);
    } else {
        bpf_trace_printk(fmt2, sizeof(fmt2), ctx->num, 0, 0);
    }
    return 0;
}
