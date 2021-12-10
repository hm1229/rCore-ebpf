clang -target bpf -Werror -O2 -c tests/demo.c -o - | llvm-objcopy --only-section=.text -O binary - user/rust/src/bin/hello.bin
