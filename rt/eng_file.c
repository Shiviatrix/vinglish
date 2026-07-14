#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <dirent.h>

char* eng_file_read(char* path) {
    if (!path) return NULL;
    FILE* f = fopen(path, "rb");
    if (!f) {
        char* err = malloc(32);
        strcpy(err, "ERROR: Cannot open file");
        return err;
    }
    
    fseek(f, 0, SEEK_END);
    long fsize = ftell(f);
    fseek(f, 0, SEEK_SET);

    char* string = malloc(fsize + 1);
    fread(string, fsize, 1, f);
    fclose(f);

    string[fsize] = 0;
    return string;
}

void eng_file_write(char* path, char* content) {
    if (!path || !content) return;
    FILE* f = fopen(path, "wb");
    if (!f) return;
    
    fwrite(content, strlen(content), 1, f);
    fclose(f);
}

char* eng_dir_list(char* path) {
    if (!path) return NULL;
    DIR *d;
    struct dirent *dir;
    d = opendir(path);
    if (!d) {
        char* err = malloc(32);
        strcpy(err, "ERROR: Cannot open dir");
        return err;
    }
    
    // We will build a simple comma-separated list of files
    int cap = 1024;
    char* result = malloc(cap);
    result[0] = '\0';
    int len = 0;
    
    while ((dir = readdir(d)) != NULL) {
        if (strcmp(dir->d_name, ".") == 0 || strcmp(dir->d_name, "..") == 0) continue;
        
        int is_dir = 0;
        if (dir->d_type == DT_DIR) {
            is_dir = 1;
        } else if (dir->d_type == DT_UNKNOWN) {
            // Fallback for some filesystems, though rare on macOS for local FS
            char full_path[1024];
            snprintf(full_path, sizeof(full_path), "%s/%s", path, dir->d_name);
            DIR* test_d = opendir(full_path);
            if (test_d) {
                is_dir = 1;
                closedir(test_d);
            }
        }
        
        int name_len = strlen(dir->d_name);
        if (len + name_len + 3 > cap) {
            cap *= 2;
            result = realloc(result, cap);
        }
        strcat(result, dir->d_name);
        if (is_dir) {
            strcat(result, "/");
            len += 1;
        }
        strcat(result, ",");
        len += name_len + 1;
    }
    closedir(d);
    
    if (len > 0) {
        result[len - 1] = '\0'; // remove last comma
    }
    return result;
}
