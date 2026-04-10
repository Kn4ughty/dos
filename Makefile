CC        := i686-elf-gcc
CFLAGS    := -ffreestanding -O2 -std=gnu99 -Wall -Wextra -MMD
LDFLAGS   := -ffreestanding -O2 -nostdlib -lgcc
AS        := i686-elf-as
ASFLAGS   := 

BUILD_DIR := build
ISO_DIR   := $(BUILD_DIR)/isodir
KERNEL    := $(BUILD_DIR)/kernel
ISO       := $(BUILD_DIR)/myos.iso

SRC := src

SRCS := $(wildcard src/*.c)
OBJS := $(patsubst src/%.c, $(BUILD_DIR)/%.o, $(SRCS))
AS_SRCS := $(wildcard src/*.s)
OBJS += $(patsubst src/%.s, $(BUILD_DIR)/%.o, $(AS_SRCS))

all: $(ISO) verify

$(BUILD_DIR):
	mkdir -p $@

$(KERNEL): $(OBJS) $(SRC)/linker.ld | $(BUILD_DIR)
	@mkdir -p $(@D) # target file directory
	$(CC) -T $(SRC)/linker.ld -o $@ $(LDFLAGS) $(OBJS)

$(BUILD_DIR)/boot.o: $(SRC)/boot.s | $(BUILD_DIR)
	# first prerequisite, output target file
	$(AS) $< -o $@ $(ASFLAGS)

$(BUILD_DIR)/%.o: $(SRC)/%.c | $(BUILD_DIR)
	$(CC) -c $< -o $@ $(CFLAGS)

$(ISO): $(KERNEL) $(SRC)/grub.cfg
	@mkdir -p $(ISO_DIR)/boot/grub
	cp $(KERNEL) $(ISO_DIR)/boot/my_os
	cp $(SRC)/grub.cfg $(ISO_DIR)/boot/grub/grub.cfg
	grub-mkrescue -o $(ISO) $(ISO_DIR)

verify: $(ISO)
	grub-file --is-x86-multiboot $(KERNEL)

run: $(ISO)
	# qemu-system-i386 -monitor stdio -m 256 -cdrom build/myos.iso
	qemu-system-i386 -cdrom $(ISO) -serial stdio

clean:
	rm -r $(BUILD_DIR)
