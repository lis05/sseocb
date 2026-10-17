SOURCES := $(wildcard *.c)
OBJECTS := $(SOURCES:.c=.o)
TARGET := common.o

CC := ../../scripts/clang-sseocb
MODE ?= guest

.PHONY: all clean

all: $(TARGET)

$(TARGET): $(OBJECTS)
	$(CC) $(MODE)-merge $^ -o $@

%.o: %.c
	$(CC) $(MODE) -I. -c $< -o $@

clean:
	rm -f $(OBJECTS) $(TARGET)
