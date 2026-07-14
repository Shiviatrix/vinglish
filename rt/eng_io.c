#include <stdio.h>
#include <stdlib.h>
#include <string.h>

char* eng_read_line() {
    char* line = NULL;
    size_t len = 0;
    ssize_t read = getline(&line, &len, stdin);
    if (read == -1) {
        free(line);
        return NULL;
    }
    // Remove newline
    if (read > 0 && line[read-1] == '\n') {
        line[read-1] = '\0';
    }
    return line;
}

int eng_str_starts_with(char* str, char* prefix) {
    if (!str || !prefix) return 0;
    return strncmp(str, prefix, strlen(prefix)) == 0;
}

char* eng_str_substring(char* str, int start) {
    if (!str) return NULL;
    int len = strlen(str);
    if (start >= len) {
        char* empty = malloc(1);
        empty[0] = '\0';
        return empty;
    }
    return strdup(str + start);
}

int eng_str_index_of(char* str, char* delimiter) {
    if (!str || !delimiter) return -1;
    char* pos = strstr(str, delimiter);
    if (pos) {
        return (int)(pos - str);
    }
    return -1;
}

char* eng_str_substring_len(char* str, int start, int length) {
    if (!str) return NULL;
    int len = strlen(str);
    if (start >= len) {
        char* empty = malloc(1);
        empty[0] = '\0';
        return empty;
    }
    char* result = malloc(length + 1);
    strncpy(result, str + start, length);
    result[length] = '\0';
    return result;
}

char* eng_str_unescape_newlines(char* str) {
    if (!str) return NULL;
    int len = strlen(str);
    char* result = malloc(len + 1);
    int j = 0;
    for (int i = 0; i < len; i++) {
        if (str[i] == '\\' && i + 1 < len && str[i+1] == 'n') {
            result[j++] = '\n';
            i++;
        } else {
            result[j++] = str[i];
        }
    }
    result[j] = '\0';
    return result;
}
