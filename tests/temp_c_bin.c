/* Generated from Vinglish SSA MIR. */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <stdbool.h>
#define print(x) _Generic((x), const char*: printf("%s", x), char*: printf("%s", x), double: printf("%g", x), bool: printf("%s", (x) ? "true" : "false"), default: printf("%ld", (long)(x)))
#define println(x) _Generic((x), const char*: printf("%s\n", x), char*: printf("%s\n", x), double: printf("%g\n", x), bool: printf("%s\n", (x) ? "true" : "false"), default: printf("%ld\n", (long)(x)))
#define abs llabs
extern const char* eng_str_concat(const char*, const char*);
extern int64_t rt_list_new(int64_t);
extern int64_t rt_list_get(int64_t, int64_t);
extern void rt_list_set(int64_t, int64_t, int64_t);
extern int64_t rt_list_len(int64_t);
extern void rt_list_push(int64_t, int64_t);
extern int64_t rt_list_pop(int64_t);

static long fn_8(uintptr_t);
int main(void);

static long fn_8(uintptr_t v_10) {
bb_8_0:
    rt_list_set(v_10, 0, 99);
    return v_10;
}

int main() {
    uintptr_t v_15 = 0;
    uintptr_t v_21 = 0;
    uintptr_t v_17 = 0;
    uintptr_t v_22 = 0;
    int64_t v_23 = 0;
    int64_t v_24 = 0;
    uintptr_t v_25 = 0;
    uintptr_t v_26 = 0;
    uintptr_t v_27 = 0;
    uintptr_t v_28 = 0;
    int64_t v_29 = 0;
    int64_t v_30 = 0;
bb_9_0:
    v_25 = rt_list_new(3);
    rt_list_push(v_25, 1);
    rt_list_push(v_25, 2);
    rt_list_push(v_25, 3);
    v_26 = v_25;
    v_27 = fn_8(v_26);
    v_28 = v_27;
    v_29 = rt_list_get(v_28, 0);
    v_30 = print(v_29);
    (void)0;
    (void)0;
    (void)0;
    return 0;
}

/* VINGLISH_MIR_PAYLOAD: eJx9UNkOgjAQLB4RjfFKvNFvABHavvkrBWnig0fE/49dmSYNopNMd3e2Oz1OrMJRq0SFkYjOggspE60zJeQxLcIokYU8ZFpqESqeq4TrOIx1mnHBozjTKj+nqfRblQ8C82npoXg873lRlh7KgaHNLWw9QT5gv5G7c17NkPI+JR0IV3W51U+xGCKODNeO3kacYWbdcNOmXutPr+3kG6dP+ha67/h72EcIUNO+seEOdcC+QX579LvQHs/L7eUak8HQiXtEa/j5VisynDo3nBouDJeGK7xgg5sFMKS5N8IMHUs= */
