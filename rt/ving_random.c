#include <stdlib.h>
#include <stdint.h>

int64_t ving_random_int() {
    return (int64_t)rand();
}

void ving_random_seed(int64_t seed) {
    srand((unsigned int)seed);
}
