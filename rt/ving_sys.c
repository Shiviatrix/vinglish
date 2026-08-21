#include <stdio.h>
#include <stdlib.h>
#include <string.h>

const char* ving_sys_env(const char* key) {
    const char* val = getenv(key);
    if (val == NULL) {
        char* empty = (char*)malloc(1);
        empty[0] = '\0';
        return empty;
    }
    char* result = (char*)malloc(strlen(val) + 1);
    strcpy(result, val);
    return result;
}

const char* ving_sys_exec(const char* cmd) {
    FILE* fp = popen(cmd, "r");
    if (fp == NULL) {
        char* err = (char*)malloc(1);
        err[0] = '\0';
        return err;
    }

    size_t cap = 1024;
    size_t len = 0;
    char* buffer = (char*)malloc(cap);

    while (1) {
        if (len + 256 > cap) {
            cap *= 2;
            buffer = (char*)realloc(buffer, cap);
        }
        size_t bytes_read = fread(buffer + len, 1, 256, fp);
        if (bytes_read == 0) {
            break;
        }
        len += bytes_read;
    }
    
    buffer[len] = '\0';
    pclose(fp);
    return buffer;
}
