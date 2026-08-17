/* Generated from Vinglish SSA MIR. */
#include <stdint.h>
#include <stddef.h>
#include <inttypes.h>
#include <stdio.h>
#include <stdlib.h>
#include <stdbool.h>
static int64_t ving_print_text(const char *value) { return fputs(value, stdout); }
static int64_t ving_print_double(double value) { return printf("%g", value); }
static int64_t ving_print_bool(bool value) { return fputs(value ? "true" : "false", stdout); }
static int64_t ving_print_i64(int64_t value) { return printf("%" PRId64, value); }
static int64_t ving_println_text(const char *value) { return printf("%s\n", value); }
static int64_t ving_println_double(double value) { return printf("%g\n", value); }
static int64_t ving_println_bool(bool value) { return fputs(value ? "true\n" : "false\n", stdout); }
static int64_t ving_println_i64(int64_t value) { return printf("%" PRId64 "\n", value); }
#define print(x) _Generic((x), const char*: ving_print_text, char*: ving_print_text, double: ving_print_double, bool: ving_print_bool, default: ving_print_i64)(x)
#define println(x) _Generic((x), const char*: ving_println_text, char*: ving_println_text, double: ving_println_double, bool: ving_println_bool, default: ving_println_i64)(x)
#define abs llabs
extern const char* ving_str_concat(const char*, const char*);
extern int64_t rt_list_new(int64_t);
extern int64_t rt_list_get(int64_t, int64_t);
extern int64_t rt_list_borrow_get(int64_t, int64_t);
extern void rt_list_set(int64_t, int64_t, int64_t);
extern int64_t rt_list_len(int64_t);
extern void rt_list_push(int64_t, int64_t);
extern int64_t rt_list_pop(int64_t);
extern void rt_list_free(int64_t);
extern uintptr_t ving_map_free(uintptr_t);


static const char *const string_literal_0 = "";

int main(void);

int main() {
    uintptr_t v_12 = 0;
    uintptr_t v_24 = 0;
    uintptr_t v_14 = 0;
    uintptr_t v_25 = 0;
    int64_t v_26 = 0;
    int64_t v_27 = 0;
    int64_t v_28 = 0;
    uintptr_t v_19 = 0;
    uintptr_t v_29 = 0;
    int64_t v_20 = 0;
    int64_t v_30 = 0;
    int64_t v_31 = 0;
    int64_t v_32 = 0;
    uintptr_t v_33 = 0;
    uintptr_t v_34 = 0;
    uintptr_t v_35 = 0;
    uintptr_t v_36 = 0;
    int64_t v_37 = 0;
    int64_t v_38 = 0;
    int64_t v_39 = 0;
    uintptr_t v_40 = 0;
    uintptr_t v_41 = 0;
    int64_t v_42 = 0;
    int64_t v_43 = 0;
    int64_t v_44 = 0;
    int64_t v_45 = 0;
bb_8_0:
    v_33 = rt_list_new(3);
    rt_list_push(v_33, 1);
    rt_list_push(v_33, 2);
    rt_list_push(v_33, 3);
    v_34 = v_33;
    v_35 = rt_list_borrow_get(v_34, 1);
    v_36 = v_35;
    v_37 = *(int64_t*)(uintptr_t)v_36;
    v_38 = print(v_37);
    v_39 = println(string_literal_0);
    v_40 = rt_list_borrow_get(v_34, 2);
    v_41 = v_40;
    *(int64_t*)(uintptr_t)v_41 = 100;
    v_42 = rt_list_get(v_34, 2);
    v_43 = v_42;
    v_44 = print(v_43);
    v_45 = println(string_literal_0);
    /* skip free v_38 */;
    /* skip free v_39 */;
    rt_list_free((int64_t)v_34);
    /* skip free v_36 */;
    /* skip free v_41 */;
    /* skip free v_43 */;
    /* skip free v_44 */;
    /* skip free v_42 */;
    /* skip free v_45 */;
    /* skip free v_37 */;
    return 0;
}

/* VINGLISH_MIR_PAYLOAD: eJyVk8tKw0AUhhOrKN7rBbTee7Ft2kKT5tLs3Ll3rYsk00BBg4q+ge8giC/g2q0L3Qo+gAvfxX/IXxkhpPTA13/mnJPTmb/NiZaG65r92PE8uxd2Pdu0XcuxfREEtuf3IsvsxiKKndD1+5GHfBgITwizG1mDXmCK2AqeCukcnfPm5Mc0N1fBMOHyr2EUO9QVcKTkOU/b5DNqTc+pTeXUCsq6rNSLoMJ9OeN7ZFS5l30LoMa9zM9z9jFzM3zm+naY3OnKsBob69zPqo2XiZ5xDxlroJFxPrWnybrsW+S6qdQFdRUYY2a1WDd43Pa4e7XY2JnsXkta6pnUOrVMrVKb1Ba1TTWoHaq0tqic387REnDAKThT9II+nYMBuAH34AE8KvoMXsAbeAev4AN8gi/wDX4Ue/5FiSp/pC2wDLaZly/DLlgHe2AD7IMDcKilf1ZpUIUG1WigNK9BswxetE1zRvELz4I4cg== */
