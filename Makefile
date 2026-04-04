all: build run

build:
	nasm boot.asm -f bin -o boot.bin

run:
	qemu-system-i386 -hda boot.bin

clean:
	rm boot.bin
