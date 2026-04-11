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

HOST_CC := gcc
HOST_CFLAGS := -std=gnu99 -Wall -Wextra -DTEST_MODE -I./src
TEST_SRCS   := $(filter-out src/boot.c src/kernel.c, $(wildcard src/*.c)) # Exclude entry points
TEST_FILES  := $(wildcard tests/test_*.c)
TEST_BINS   := $(patsubst tests/test_%.c, $(BUILD_DIR)/test_%, $(TEST_FILES))

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

$(BUILD_DIR)/test_%: tests/test_%.c  tests/host_stubs.c $(TEST_SRCS) | $(BUILD_DIR)
	$(HOST_CC) $(HOST_CFLAGS) $^ -o $@

$(ISO): $(KERNEL) $(SRC)/grub.cfg
	@mkdir -p $(ISO_DIR)/boot/grub
	cp $(KERNEL) $(ISO_DIR)/boot/my_os
	cp $(SRC)/grub.cfg $(ISO_DIR)/boot/grub/grub.cfg
	grub-mkrescue -o $(ISO) $(ISO_DIR)

verify: $(ISO) test
	grub-file --is-x86-multiboot $(KERNEL)

test: $(TEST_BINS)
	@for test in $(TEST_BINS); do\
		echo "running $$test..";\
		./$$test || exit 1; \
	done
	@echo "All tests passed!"

run: $(ISO)
	# qemu-system-i386 -monitor stdio -m 256 -cdrom build/myos.iso
	qemu-system-i386 -cdrom $(ISO) -serial stdio

clean:
	rm -r $(BUILD_DIR)
