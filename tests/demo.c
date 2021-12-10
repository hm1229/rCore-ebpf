#define size_t unsigned long int

static long (*bpf_trace_printk)(const char* fmt, int fmt_size, long p1, long p2, long p3) = (void*)6;

struct GeneralRegs {
  size_t zero;
  size_t ra;
  size_t sp;
  size_t gp;
  size_t tp;
  size_t t0;
  size_t t1;
  size_t t2;
  size_t s0;
  size_t s1;
  size_t a0;
  size_t a1;
  size_t a2;
  size_t a3;
  size_t a4;
  size_t a5;
  size_t a6;
  size_t a7;
  size_t s2;
  size_t s3;
  size_t s4;
  size_t s5;
  size_t s6;
  size_t s7;
  size_t s8;
  size_t s9;
  size_t s10;
  size_t s11;
  size_t t3;
  size_t t4;
  size_t t5;
  size_t t6;
};

struct TrapFrame {
    struct GeneralRegs general;
    size_t sstatus;
    size_t sepc;
};

int prog(struct TrapFrame* cx) {
    char fmt[] = "from ebpf: the probed address is {}";
    bpf_trace_printk(fmt, 37, cx->general.a1, 0, 0);
    return 0;
}
