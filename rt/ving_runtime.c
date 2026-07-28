#include <stdlib.h>
#include <stdint.h>
#include <string.h>

void* ving_alloc(int64_t size) {
    return malloc(size);
}

void ving_free(void* ptr) {
    free(ptr);
}

void ving_write(int64_t* ptr, int64_t index, int64_t val) {
    ptr[index] = val;
}

int64_t ving_read(int64_t* ptr, int64_t index) {
    return ptr[index];
}

#include <stdio.h>
void print_num(int64_t n) {
    printf("%lld\n", (long long)n);
}
