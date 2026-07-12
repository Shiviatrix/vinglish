#include <stdlib.h>
#include <string.h>
#include <stdint.h>

struct String {
    char* ptr;
};

struct String eng_string_new(char* ptr) {
    struct String s;
    if (!ptr) {
        s.ptr = calloc(1, 1);
        return s;
    }
    s.ptr = strdup(ptr);
    return s;
}

int64_t eng_string_len(struct String s) {
    if (!s.ptr) return 0;
    return strlen(s.ptr);
}

struct String eng_string_concat(struct String a, struct String b) {
    struct String res;
    if (!a.ptr && !b.ptr) {
        res.ptr = calloc(1, 1);
        return res;
    }
    size_t len_a = a.ptr ? strlen(a.ptr) : 0;
    size_t len_b = b.ptr ? strlen(b.ptr) : 0;
    char* new_str = malloc(len_a + len_b + 1);
    if (len_a > 0) memcpy(new_str, a.ptr, len_a);
    if (len_b > 0) memcpy(new_str + len_a, b.ptr, len_b);
    new_str[len_a + len_b] = '\0';
    res.ptr = new_str;
    return res;
}

void eng_string_free(struct String s) {
    if (s.ptr) {
        free(s.ptr);
    }
}
