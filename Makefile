ROOT := $($(pwd))
CC = i686-elf-gcc

all: build verify run

build:
	mkdir -p ./build
	i686-elf-as boot.s -o build/boot.o
	$(CC) -c kernel.c -o build/kernel.o -std=gnu99 -ffreestanding -O2 -Wall -Wextra
	# cd build
	i686-elf-gcc -T linker.ld -o build/kernel -ffreestanding -O2 -nostdlib build/boot.o build/kernel.o -lgcc
	# Make disk image
	mkdir -p build/isodir/boot/grub
	cp build/kernel build/isodir/boot/my_os
	cp grub.cfg build/isodir/boot/grub/grub.cfg
	grub-mkrescue -o build/myos.iso build/isodir

verify: build
	grub-file --is-x86-multiboot build/myos.iso

run:
	qemu-system-i386 -monitor stdio -m 256 -cdrom build/myos.iso
	qemu-system-i386 -cdrom myos.iso

clean:
	rm -r build/
