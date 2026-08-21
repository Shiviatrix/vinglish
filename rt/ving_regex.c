#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <regex.h>
#include <stdint.h>

int64_t ving_regex_is_match(const char* pattern, const char* text) {
    regex_t regex;
    int reti;

    reti = regcomp(&regex, pattern, REG_EXTENDED);
    if (reti) {
        return 0; // Failed to compile
    }

    reti = regexec(&regex, text, 0, NULL, 0);
    regfree(&regex);

    if (!reti) {
        return 1; // Match
    } else {
        return 0; // No match
    }
}

const char* ving_regex_replace(const char* pattern, const char* text, const char* replacement) {
    regex_t regex;
    int reti;
    regmatch_t pmatch[1];

    reti = regcomp(&regex, pattern, REG_EXTENDED);
    if (reti) {
        char* res = (char*)malloc(strlen(text) + 1);
        strcpy(res, text);
        return res;
    }

    // Only replacing the first match for simplicity in this implementation
    reti = regexec(&regex, text, 1, pmatch, 0);
    if (!reti) {
        size_t len_before = pmatch[0].rm_so;
        size_t len_after = strlen(text) - pmatch[0].rm_eo;
        size_t len_rep = strlen(replacement);
        
        char* res = (char*)malloc(len_before + len_rep + len_after + 1);
        strncpy(res, text, len_before);
        strcpy(res + len_before, replacement);
        strcpy(res + len_before + len_rep, text + pmatch[0].rm_eo);
        
        regfree(&regex);
        return res;
    }

    regfree(&regex);
    char* res = (char*)malloc(strlen(text) + 1);
    strcpy(res, text);
    return res;
}
