#include <stdlib.h>
#include <string.h>
#include <stdint.h>

#define INITIAL_CAPACITY 16

typedef struct Entry {
    char* key;
    int64_t value;
    struct Entry* next;
} Entry;

typedef struct {
    Entry** buckets;
    int64_t capacity;
    int64_t size;
} Map;

static uint64_t hash_string(const char* str) {
    uint64_t hash = 5381;
    int c;
    while ((c = *str++)) {
        hash = ((hash << 5) + hash) + c;
    }
    return hash;
}

void* eng_map_new() {
    Map* map = (Map*)malloc(sizeof(Map));
    map->capacity = INITIAL_CAPACITY;
    map->size = 0;
    map->buckets = (Entry**)calloc(INITIAL_CAPACITY, sizeof(Entry*));
    return map;
}

void eng_map_insert(void* map_ptr, const char* key, int64_t value) {
    Map* map = (Map*)map_ptr;
    uint64_t hash = hash_string(key);
    int64_t index = hash % map->capacity;

    Entry* current = map->buckets[index];
    while (current) {
        if (strcmp(current->key, key) == 0) {
            current->value = value; // update
            return;
        }
        current = current->next;
    }

    Entry* new_entry = (Entry*)malloc(sizeof(Entry));
    new_entry->key = strdup(key);
    new_entry->value = value;
    new_entry->next = map->buckets[index];
    map->buckets[index] = new_entry;
    map->size++;

    // For simplicity, we skip rehashing in this minimal runtime
}

int64_t eng_map_get(void* map_ptr, const char* key) {
    Map* map = (Map*)map_ptr;
    uint64_t hash = hash_string(key);
    int64_t index = hash % map->capacity;

    Entry* current = map->buckets[index];
    while (current) {
        if (strcmp(current->key, key) == 0) {
            return current->value;
        }
        current = current->next;
    }
    return 0; // return 0 if not found
}

void eng_map_free(void* map_ptr) {
    Map* map = (Map*)map_ptr;
    for (int64_t i = 0; i < map->capacity; i++) {
        Entry* current = map->buckets[i];
        while (current) {
            Entry* next = current->next;
            free(current->key);
            free(current);
            current = next;
        }
    }
    free(map->buckets);
    free(map);
}
