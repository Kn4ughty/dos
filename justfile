arch := "x86_64"
kernel := "build/kernel-" + arch + ".bin"
iso_file := "build/os-" + arch + ".iso"
target := arch+ "-target"
rust_os := "target/" + target + "/debug/libos.a"

asm_folder := "src/arch/" + arch + "/"
linker_script := "src/arch/" + arch + "/linker.ld"
grub_cfg := "src/arch/" + arch+ "/grub.cfg"

all: iso

clean:
    rm -rf build
    # cargo clean

run: iso
    qemu-system-x86_64 -cdrom {{iso_file}} -serial stdio

dbg: iso
    qemu-system-x86_64 -cdrom build/os-x86_64.iso -serial stdio -d int,cpu_reset -no-reboot -no-shutdown -s -S


iso: kernel
    @mkdir -p build/isofiles/boot/grub
    @cp {{kernel}} build/isofiles/boot/kernel.bin
    @cp {{grub_cfg}} build/isofiles/boot/grub/grub.cfg
    @grub-mkrescue -o {{iso_file}} build/isofiles 2> /dev/null
    @rm -rf build/isofiles # required?

kernel: (compile-asm)
    cargo build --lib
    # todo. Remove no warn
    ld -n --no-warn-rwx-segments -T {{linker_script}} -o {{kernel}} build/arch/{{arch}}/*.o {{rust_os}}
    @grub-file --is-x86-multiboot2 {{kernel}}

[private]
compile-asm:
    #!/usr/bin/env bash
    set -e
    mkdir -p build/arch/{{arch}}
    for file in src/arch/{{arch}}/*.asm; do
        nasm -felf64 "$file" -i {{asm_folder}} -o "build/arch/{{arch}}/$(basename "${file%.asm}.o")";
    done
