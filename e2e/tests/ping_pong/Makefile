COMMON_DIR := ../../common
COMMON_MODE ?= guest
CC := ../../../scripts/clang-sseocb

GUEST_SOURCES := guest.c
NATIVE_SOURCES := $(wildcard native.c)
COMMON_SOURCES := $(wildcard $(COMMON_DIR)/*.c)

GUEST_OBJS := $(patsubst %.c,artifacts/%.guest.o,$(GUEST_SOURCES))
NATIVE_OBJS := $(patsubst %.c,artifacts/%.native.o,$(NATIVE_SOURCES))
COMMON_OBJS := $(patsubst $(COMMON_DIR)/%.c,artifacts/common/%.o,$(COMMON_SOURCES))

ifeq ($(COMMON_MODE),guest)
GUEST_MERGE_INPUTS := $(GUEST_OBJS) artifacts/common.o
NATIVE_MERGE_INPUTS := $(NATIVE_OBJS)
else
GUEST_MERGE_INPUTS := $(GUEST_OBJS)
NATIVE_MERGE_INPUTS := $(NATIVE_OBJS) artifacts/common.o
endif

TARGETS := artifacts/guest.o
ifneq ($(strip $(NATIVE_MERGE_INPUTS)),)
TARGETS += artifacts/guest-native.o
endif

.PHONY: all common clean

all: $(TARGETS)

common: artifacts/common.o

artifacts/common/%.o: $(COMMON_DIR)/%.c
	@mkdir -p $(dir $@)
	$(CC) $(COMMON_MODE) -I$(COMMON_DIR) -I. -c $< -o $@

artifacts/common.o: $(COMMON_OBJS)
	@mkdir -p artifacts
	$(CC) $(COMMON_MODE)-merge $^ -o $@

artifacts/%.guest.o: %.c
	@mkdir -p $(dir $@)
	$(CC) guest -I$(COMMON_DIR) -I. -c $< -o $@

artifacts/%.native.o: %.c
	@mkdir -p $(dir $@)
	$(CC) native -I$(COMMON_DIR) -I. -c $< -o $@

artifacts/guest.o: $(GUEST_MERGE_INPUTS)
	@mkdir -p artifacts
	$(CC) guest-merge $(GUEST_MERGE_INPUTS) -o $@

artifacts/guest-native.o: $(NATIVE_MERGE_INPUTS)
	@mkdir -p artifacts
	$(CC) native-merge $(NATIVE_MERGE_INPUTS) -o $@

clean:
	rm -rf artifacts
