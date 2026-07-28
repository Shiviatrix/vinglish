#include <pthread.h>
#include <stdlib.h>
#include <unistd.h>
#include <stdint.h>

// Note: In Vinglish, generic function types or pointers are passed as void*
// spawn<T>(function() -> T f) returns Thread<T>
// We'll treat the function pointer as void* (*)(void)

struct ThreadStruct {
    pthread_t handle;
};

int64_t thread_spawn(void* (*f)(void)) {
    pthread_t* t = malloc(sizeof(pthread_t));
    pthread_create(t, NULL, (void* (*)(void*))f, NULL);
    return (int64_t)t;
}

void* thread_join(int64_t handle_ptr) {
    pthread_t* t = (pthread_t*)handle_ptr;
    void* result;
    pthread_join(*t, &result);
    free(t);
    return result;
}

void thread_sleep(int64_t milliseconds) {
    usleep(milliseconds * 1000);
}
