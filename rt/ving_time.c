#include <time.h>
#include <stdint.h>
#include <unistd.h>
#ifdef _WIN32
#include <windows.h>
#endif

int64_t ving_time_now() {
    return (int64_t)time(NULL);
}

void ving_time_sleep(int64_t ms) {
#ifdef _WIN32
    Sleep((DWORD)ms);
#else
    usleep((useconds_t)(ms * 1000));
#endif
}
