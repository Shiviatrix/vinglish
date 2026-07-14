#include <termios.h>
#include <unistd.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/ioctl.h>

static struct termios orig_termios;

void term_disable_raw_mode() {
    tcsetattr(STDIN_FILENO, TCSAFLUSH, &orig_termios);
}

void term_enable_raw_mode() {
    tcgetattr(STDIN_FILENO, &orig_termios);
    atexit(term_disable_raw_mode);

    struct termios raw = orig_termios;
    raw.c_iflag &= ~(BRKINT | ICRNL | INPCK | ISTRIP | IXON);
    raw.c_oflag &= ~(OPOST);
    raw.c_cflag |= (CS8);
    raw.c_lflag &= ~(ECHO | ICANON | IEXTEN | ISIG);
    raw.c_cc[VMIN] = 0;
    raw.c_cc[VTIME] = 1;

    tcsetattr(STDIN_FILENO, TCSAFLUSH, &raw);
}

void term_clear_screen() {
    write(STDOUT_FILENO, "\x1b[2J", 4);
    write(STDOUT_FILENO, "\x1b[H", 3);
}

long term_read_key() {
    int nread;
    char c;
    while ((nread = read(STDIN_FILENO, &c, 1)) != 1) {
        if (nread == -1) return 0;
    }
    
    // Handle escape sequences
    if (c == '\x1b') {
        char seq[3];
        if (read(STDIN_FILENO, &seq[0], 1) != 1) return '\x1b';
        if (read(STDIN_FILENO, &seq[1], 1) != 1) return '\x1b';
        
        if (seq[0] == '[') {
            switch (seq[1]) {
                case 'A': return 1000; // Up
                case 'B': return 1001; // Down
                case 'C': return 1002; // Right
                case 'D': return 1003; // Left
            }
        }
        return '\x1b';
    } else {
        return c;
    }
}

void term_move_cursor(long row, long col) {
    char buf[32];
    snprintf(buf, sizeof(buf), "\x1b[%ld;%ldH", row, col);
    write(STDOUT_FILENO, buf, strlen(buf));
}

typedef struct TermSize {
    long rows;
    long cols;
} TermSize_t;

TermSize_t term_get_size() {
    struct winsize ws;
    TermSize_t ts;
    if (ioctl(STDOUT_FILENO, TIOCGWINSZ, &ws) == -1 || ws.ws_col == 0) {
        ts.rows = 24;
        ts.cols = 80;
    } else {
        ts.rows = ws.ws_row;
        ts.cols = ws.ws_col;
    }
    return ts;
}
