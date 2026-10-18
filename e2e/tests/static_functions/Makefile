COMMON_DIR := ../../common
COMMON_MODE ?= guest
CC := ../../../scripts/clang-sseocb

GUEST_SOURCES := guest.c dir/guest.c
NATIVE_SOURCES := $(wildcard native.c)

GUEST_OBJS := $(patsubst %.c,artifacts/%.guest.o,$(GUEST_SOURCES))
NATIVE_OBJS := $(patsubst %.c,artifacts/%.native.o,$(NATIVE_SOURCES))

ifeq ($(COMMON_MODE),guest)
GUEST_MERGE_INPUTS := $(GUEST_OBJS) $(COMMON_DIR)/common.o
NATIVE_MERGE_INPUTS := $(NATIVE_OBJS)
else
GUEST_MERGE_INPUTS := $(GUEST_OBJS)
NATIVE_MERGE_INPUTS := $(NATIVE_OBJS) $(COMMON_DIR)/common.o
endif

TARGETS := artifacts/guest.o
ifneq ($(strip $(NATIVE_MERGE_INPUTS)),)
TARGETS += artifacts/guest-native.o
endif

.PHONY: all common clean

all: $(TARGETS)

common:
	$(MAKE) -C $(COMMON_DIR) clean
	$(MAKE) -C $(COMMON_DIR) MODE=$(COMMON_MODE)

artifacts/%.guest.o: %.c
	@mkdir -p $(dir $@)
	$(CC) guest -I$(COMMON_DIR) -I. -c $< -o $@

artifacts/%.native.o: %.c
	@mkdir -p $(dir $@)
	$(CC) native -I$(COMMON_DIR) -I. -c $< -o $@

artifacts/guest.o: $(GUEST_OBJS) common
	@mkdir -p artifacts
	$(CC) guest-merge $(GUEST_MERGE_INPUTS) -o $@

artifacts/guest-native.o: $(NATIVE_OBJS) common
	@mkdir -p artifacts
	$(CC) native-merge $(NATIVE_MERGE_INPUTS) -o $@

clean:
	rm -rf artifacts
	$(MAKE) -C $(COMMON_DIR) clean
