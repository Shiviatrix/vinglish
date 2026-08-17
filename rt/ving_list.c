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

typedef struct {
    int64_t len;
    int64_t cap;
    int64_t* data;
} rt_list_t;

int64_t rt_list_new(int64_t capacity) {
    if (capacity < 4) capacity = 4;
    rt_list_t* list = (rt_list_t*)malloc(sizeof(rt_list_t));
    list->len = 0;
    list->cap = capacity;
    list->data = (int64_t*)malloc(sizeof(int64_t) * capacity);
    return (int64_t)list;
}

int64_t rt_list_get(int64_t list_ptr, int64_t idx) {
    rt_list_t* list = (rt_list_t*)list_ptr;
    if (idx < 0 || idx >= list->len) {
        printf("Index out of bounds\\n");
        exit(1);
    }
    return list->data[idx];
}

int64_t rt_list_borrow_get(int64_t list_ptr, int64_t idx) {
    rt_list_t* list = (rt_list_t*)list_ptr;
    if (idx < 0 || idx >= list->len) {
        printf("Index out of bounds\\n");
        exit(1);
    }
    return (int64_t)&list->data[idx];
}

void rt_list_set(int64_t list_ptr, int64_t idx, int64_t val) {
    rt_list_t* list = (rt_list_t*)list_ptr;
    if (idx < 0 || idx >= list->len) {
        printf("Index out of bounds\\n");
        exit(1);
    }
    list->data[idx] = val;
}

int64_t rt_list_len(int64_t list_ptr) {
    rt_list_t* list = (rt_list_t*)list_ptr;
    return list->len;
}

void rt_list_push(int64_t list_ptr, int64_t val) {
    rt_list_t* list = (rt_list_t*)list_ptr;
    if (list->len >= list->cap) {
        list->cap *= 2;
        list->data = (int64_t*)realloc(list->data, sizeof(int64_t) * list->cap);
    }
    list->data[list->len++] = val;
}

int64_t rt_list_pop(int64_t list_ptr) {
    rt_list_t* list = (rt_list_t*)list_ptr;
    if (list->len == 0) {
        printf("Pop from empty list\\n");
        exit(1);
    }
    return list->data[--list->len];
}

void rt_list_free(int64_t list_ptr) {
    rt_list_t* list = (rt_list_t*)list_ptr;
    if (!list) return;
    free(list->data);
    free(list);
}
