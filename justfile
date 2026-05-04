arch := "x86_64"
kernel := "build/kernel-" + arch + ".bin"
iso_file := "build/os-" + arch + ".iso"

linker_script := "src/arch/" + arch + "/linker.ld"
grub_cfg := "src/arch/" + arch+ "/grub.cfg"

all: iso

clean:
    rm -rf build

run: iso
    qemu-system-x86_64 -cdrom {{iso_file}}

iso: kernel
    @mkdir -p build/isofiles/boot/grub
    @cp {{kernel}} build/isofiles/boot/kernel.bin
    @cp {{grub_cfg}} build/isofiles/boot/grub/grub.cfg
    @grub-mkrescue -o {{iso_file}} build/isofiles 2> /dev/null
    @rm -rf build/isofiles # required?

kernel: (compile-asm)
    ld -n -T {{linker_script}} -o {{kernel}} build/arch/{{arch}}/*.o

[private]
compile-asm:
    #!/usr/bin/env bash
    mkdir -p build/arch/{{arch}}
    for file in src/arch/{{arch}}/*.asm; do\
        nasm -felf64 "$file" -o "build/arch/{{arch}}/$(basename "${file%.asm}.o")"; \
    done
