#include <stdlib.h>
#include <stdint.h>
#include <stdio.h>

int64_t rt_alloc(int64_t size) {
    return (int64_t)malloc(size);
}

void rt_write(int64_t ptr, int64_t value) {
    *(int64_t*)ptr = value;
}

int64_t rt_read(int64_t ptr) {
    return *(int64_t*)ptr;
}
