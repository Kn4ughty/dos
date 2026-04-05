all: build run

build:
	nasm boot.asm -f bin -o boot.bin

run:
	qemu-system-x86_64 -monitor stdio -m 256 -drive format=raw,file=boot.bin

clean:
	rm boot.bin
