all: build run

build:
	nasm boot.asm -f bin -o boot.bin

run:
	qemu-system-x86_64 -m 256 -hda boot.bin

clean:
	rm boot.bin
