#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/socket.h>
#include <arpa/inet.h>
#include <netdb.h>
#include <stdint.h>

int64_t ving_net_tcp_connect(const char* host, int64_t port) {
    int sock;
    struct sockaddr_in server;
    struct hostent *he;

    if ((he = gethostbyname(host)) == NULL) {
        return -1;
    }

    if ((sock = socket(AF_INET, SOCK_STREAM, 0)) < 0) {
        return -1;
    }

    server.sin_family = AF_INET;
    server.sin_port = htons(port);
    server.sin_addr = *((struct in_addr *)he->h_addr);
    memset(&(server.sin_zero), '\0', 8);

    if (connect(sock, (struct sockaddr *)&server, sizeof(struct sockaddr)) < 0) {
        close(sock);
        return -1;
    }

    return (int64_t)sock;
}

int64_t ving_net_tcp_send(int64_t sock, const char* data) {
    int len = strlen(data);
    int sent = send((int)sock, data, len, 0);
    return (int64_t)sent;
}

const char* ving_net_tcp_recv(int64_t sock, int64_t max_bytes) {
    char* buffer = (char*)malloc(max_bytes + 1);
    int received = recv((int)sock, buffer, max_bytes, 0);
    if (received < 0) {
        free(buffer);
        return "";
    }
    buffer[received] = '\0';
    return buffer;
}

void ving_net_tcp_close(int64_t sock) {
    close((int)sock);
}
