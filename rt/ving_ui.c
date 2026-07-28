#include <stdio.h>
#include <unistd.h>
#include <string.h>

typedef struct Color {
    long r;
    long g;
    long b;
} Color_t;

void ui_set_fg(Color_t c) {
    char buf[64];
    snprintf(buf, sizeof(buf), "\x1b[38;2;%ld;%ld;%ldm", c.r, c.g, c.b);
    write(STDOUT_FILENO, buf, strlen(buf));
}

void ui_set_bg(Color_t c) {
    char buf[64];
    snprintf(buf, sizeof(buf), "\x1b[48;2;%ld;%ld;%ldm", c.r, c.g, c.b);
    write(STDOUT_FILENO, buf, strlen(buf));
}

void ui_reset_color() {
    write(STDOUT_FILENO, "\x1b[0m", 4);
}
