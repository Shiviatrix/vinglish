#include <stdlib.h>
#include <stdio.h>
#include <unistd.h>
#include <sys/wait.h>
#include <string.h>

// Vinglish Vector_t structure definition
typedef struct Vector {
    long* data;
    long len;
    long capacity;
} Vector_t;

typedef struct Process {
    long pid;
    long fd;
} Process_t;

Process_t spawn_process(char* command, Vector_t* args) {
    // Basic fork/exec implementation
    // We will pipe stdout and stderr
    int pipefd[2];
    Process_t err_proc = {-1, -1};
    if (pipe(pipefd) == -1) {
        return err_proc; // Error
    }
    
    pid_t pid = fork();
    if (pid == -1) {
        return err_proc; // Error
    }
    
    if (pid == 0) {
        // Child
        close(pipefd[0]);
        dup2(pipefd[1], STDOUT_FILENO);
        dup2(pipefd[1], STDERR_FILENO);
        close(pipefd[1]);
        
        // args->data contains long* which are actually char* in this case
        // We need to build a char** for execvp
        char** argv = malloc((args->len + 2) * sizeof(char*));
        argv[0] = command;
        for (int i = 0; i < args->len; i++) {
            argv[i + 1] = (char*)args->data[i];
        }
        argv[args->len + 1] = NULL;
        
        execvp(command, argv);
        exit(1); // If execvp fails
    } else {
        // Parent
        close(pipefd[1]);
        // For simplicity, we just return the pipe read FD as the 'pid'
        // In a real system, we'd return a struct containing both pid and fd.
        // Let's pack them into a malloc'd struct since Process just has 'pid' as a number.
        // Actually, let's just return the fd for now so we can read it.
        // Wait, the process_wait needs the PID!
        
        Process_t proc;
        proc.pid = pid;
        proc.fd = pipefd[0];
        return proc;
    }
}

// ProcessResult type has exit_code, stdout, stderr (all numbers/strings)
typedef struct ProcessResult {
    long exit_code;
    char* stdout_str;
    char* stderr_str;
} ProcessResult_t;

ProcessResult_t process_wait(Process_t* p_ptr) {
    pid_t pid = p_ptr->pid;
    
    int status;
    waitpid(pid, &status, 0);
    
    ProcessResult_t res;
    if (WIFEXITED(status)) {
        res.exit_code = WEXITSTATUS(status);
    } else {
        res.exit_code = -1;
    }
    res.stdout_str = "";
    res.stderr_str = "";
    return res;
}

char* process_read_output(Process_t* p_ptr) {
    int fd = p_ptr->fd;
    
    char* buf = malloc(4096);
    int total = 0;
    while (1) {
        int r = read(fd, buf + total, 4096 - total - 1);
        if (r <= 0) break;
        total += r;
        // Basic realloc not implemented here for simplicity, assuming < 4096 output
    }
    buf[total] = '\0';
    return buf;
}
