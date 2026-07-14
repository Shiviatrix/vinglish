#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>

struct String {
    char* ptr;
};

// ANSI terminal UI backend for Vinglish Studio

void ving_ui_init() {
    // Clear screen, hide cursor, alternative screen buffer
    printf("\x1b[?1049h\x1b[?25l\x1b[2J\x1b[H");
    fflush(stdout);
}

void ving_ui_cleanup() {
    // Show cursor, normal screen buffer, reset attributes
    printf("\x1b[0m\x1b[?25h\x1b[?1049l");
    fflush(stdout);
}

// Draw a filled rectangle with background color
void ving_ui_draw_rect(int64_t x, int64_t y, int64_t w, int64_t h, int64_t r, int64_t g, int64_t b) {
    // Convert 1400x900 pixel coordinates to 140x45 grid (approx)
    int col = (x / 10) + 1;
    int row = (y / 20) + 1;
    int cols = w / 10;
    int rows = h / 20;

    if (cols < 1) cols = 1;
    if (rows < 1) rows = 1;

    for (int i = 0; i < rows; i++) {
        // Move cursor to (row+i, col), set background color
        printf("\x1b[%d;%dH\x1b[48;2;%lld;%lld;%lldm", row + i, col, r, g, b);
        for (int j = 0; j < cols; j++) {
            putchar(' '); // fill with spaces
        }
    }
    printf("\x1b[0m"); // reset
    fflush(stdout);
}

// Draw text with foreground color at a specific position
void ving_ui_draw_text(int64_t x, int64_t y, struct String text, int64_t fg_r, int64_t fg_g, int64_t fg_b, int64_t bg_r, int64_t bg_g, int64_t bg_b, int64_t transparent_bg) {
    if (!text.ptr) return;
    
    int col = (x / 10) + 1;
    int row = (y / 20) + 1;

    // Move cursor, set fg color
    printf("\x1b[%d;%dH\x1b[38;2;%lld;%lld;%lldm", row, col, fg_r, fg_g, fg_b);
    
    // Set bg color if not transparent
    if (!transparent_bg) {
        printf("\x1b[48;2;%lld;%lld;%lldm", bg_r, bg_g, bg_b);
    }
    
    printf("%s\x1b[0m", text.ptr);
    fflush(stdout);
}
