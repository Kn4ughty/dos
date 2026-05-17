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

test: (compile-asm "-dTEST_BUILD")
    #!/usr/bin/env bash
    # set -euo pipefail
    cargo build --tests

    for file in tests/*.rs; do
        test_name=$(basename "${file%.*}")
        echo "Running test $test_name..."

        # cargo 




        # cargo test --test "$test_name" --no-run --message-format=json -- --emit=obj > build/cargo_output.json
        # obj_file=$(jq -r 'select(.profile.test == true and .target.name == "'"$test_name"'") | .filenames[] | select(.endswith(".o"))' build/cargo_output.json)
        # echo $obj_file


        # cargo rustc --test "$test_name" --message-format=json -- --emit=obj > build/cargo_output.json
        # exec=$(cat build/cargo_output.json | jq -r 'select(.profile.test == true and .target.name == "basic_boot") | .executable')
        # obj=$(jq -r 'select(.reason=="compiler-artifact") | .filenames[] | select(endswith(".o"))' build/cargo_output.json | head -n1)

        # echo $exec

        # test_kernel="build/kernel-test-$test_name.bin"
        # ld -n --no-warn-rwx-segments -T {{linker_script}} -o "$test_kernel" build/arch/{{arch}}/*.o "$exec"
        #
        # echo $exec
        # # echo {{iso_file}}
        # #
        # just package_iso_direct "$test_kernel" {{iso_file}}
        #
        # qemu-system-x86_64 -cdrom {{iso_file}} -serial stdio

    done


iso: (compile-asm) 
    cargo build
    just package_iso {{rust_os}} {{kernel}} {{iso_file}}
    # @grub-file --is-x86-multiboot2 {{kernel}}

[private]
compile-asm nasm_flags="":
    #!/usr/bin/env bash
    set -e
    #echo {{nasm_flags}}
    mkdir -p build/arch/{{arch}}
    for file in src/arch/{{arch}}/*.asm; do
        nasm -felf64 {{nasm_flags}} "$file" -i {{asm_folder}} -o "build/arch/{{arch}}/$(basename "${file%.asm}.o")";
    done


package_iso_direct input_artifact output_iso:
    # todo. Remove no warn

    mkdir -p build/isofiles/boot/grub

    cp {{input_artifact}} build/isofiles/boot/kernel.bin
    cp {{grub_cfg}} build/isofiles/boot/grub/grub.cfg
    grub-mkrescue -o {{output_iso}} build/isofiles 2> /dev/null
    rm -rf build/isofiles # required?

package_iso input_artifact output_bin output_iso:
    # todo. Remove no warn
    ld -n --no-warn-rwx-segments -T {{linker_script}} -o {{output_bin}} build/arch/{{arch}}/*.o {{input_artifact}}

    mkdir -p build/isofiles/boot/grub

    cp {{output_bin}} build/isofiles/boot/kernel.bin
    cp {{grub_cfg}} build/isofiles/boot/grub/grub.cfg
    grub-mkrescue -o {{output_iso}} build/isofiles 2> /dev/null
    rm -rf build/isofiles # required?

