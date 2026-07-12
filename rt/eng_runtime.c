#include <stdlib.h>
#include <stdint.h>
#include <string.h>

void* eng_alloc(int64_t size) {
    return malloc(size);
}

void eng_free(void* ptr) {
    free(ptr);
}

void eng_write(int64_t* ptr, int64_t index, int64_t val) {
    ptr[index] = val;
}

int64_t eng_read(int64_t* ptr, int64_t index) {
    return ptr[index];
}
