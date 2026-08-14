#include <stdio.h>
#include <stdlib.h>

void ving_test_fail(const char* msg) {
    fprintf(stderr, "TEST FAILURE: %s\n", msg);
    exit(1);
}
