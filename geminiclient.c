// Based on ssl example by Alexey Kutepov <reximkut@gmail.com>

#define _POSIX_C_SOURCE 200112L

#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <errno.h>
#include <stdbool.h>

#include <netdb.h>
#include <sys/types.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <arpa/inet.h>
#include <unistd.h>

#include <openssl/ssl.h>
#include <openssl/err.h>

#define HOST "geminiprotocol.net"
#define PORT "1965"

// gemini://geminiprotocol.net/
#define PROTOCAL "gemini://"

char* get_hostname(char *url)
{
    size_t prot_len = strlen(PROTOCAL);
    if(strncmp(url, PROTOCAL, prot_len) != 0)
    {
        fprintf(stderr, 
                "ERROR: malformed request url provided: no begining protocal %s\n",
                url
               );
        return NULL;
    }
    char* begin_host = url+prot_len;
    char* end_host = strchr(begin_host, '/');
    if(end_host == NULL)
    {
        fprintf(stderr, 
                "ERROR: malformed request url provided: no terminating slash %s\n",
                url
               );
        return NULL;
    }
    size_t len = end_host - begin_host;
    return strndup(begin_host, len);
}

char *read_ssl_to_string(SSL *ssl)
{
    size_t capacity = 1024;
    size_t size = 0;
    char *buffer = malloc(capacity);
    if (!buffer) return NULL;
    char temp[1024];
    int n;
    while ((n = SSL_read(ssl, temp, sizeof(temp))) > 0) {
        if (size + n + 1 > capacity) {
            capacity *= 2; 
            char *new_buf = realloc(buffer, capacity);
            if (!new_buf) {
                free(buffer);
                return NULL;
            }
            buffer = new_buf;
        }
        memcpy(buffer + size, temp, n);
        size += n;
    }
    buffer[size] = '\0';
    return buffer;
}

char *make_request(char* url)
{
    char* hostname = get_hostname(url);
    struct addrinfo hints = {0};
    hints.ai_family = AF_INET;
    hints.ai_socktype = SOCK_STREAM;
    hints.ai_protocol = IPPROTO_TCP;

    struct addrinfo *addrs;
    if (getaddrinfo(hostname, PORT, &hints, &addrs) < 0) {
        fprintf(stderr, "Could not get address of `%s`: %s\n",
                hostname,
                strerror(errno));
        free(hostname);
        exit(1);
    }
    free(hostname);

    int sd = 0;
    for (struct addrinfo *addr = addrs; addr != NULL; addr = addr->ai_next) {
        sd = socket(addr->ai_family, addr->ai_socktype, addr->ai_protocol);

        if (sd == -1) break;
        if (connect(sd, addr->ai_addr, addr->ai_addrlen) == 0) break;

        close(sd);
        sd = -1;
    }
    freeaddrinfo(addrs);

    if (sd == -1) {
        fprintf(stderr, "Could not connect to "HOST":"PORT": %s\n",
                strerror(errno));
        exit(1);
    }

    OpenSSL_add_all_algorithms();
    SSL_load_error_strings();
    SSL_CTX *ctx = SSL_CTX_new(TLS_client_method());

    if (ctx == NULL) {
        fprintf(stderr, "ERROR: could not initialize the SSL context: %s\n",
                strerror(errno));
        exit(1);
    }

    SSL *ssl = SSL_new(ctx);
    SSL_set_fd(ssl, sd);

    if (SSL_connect(ssl) < 0) {
        fprintf(stderr, "ERROR: could not connect via SSL: %s\n",
                strerror(errno));
        exit(1);
    }
    char request[1024] = {0};
    sprintf(request, "%s\r\n", url);
    SSL_write(ssl, request, strlen(request));
    char *buffer = read_ssl_to_string(ssl);
    SSL_set_shutdown(ssl, SSL_RECEIVED_SHUTDOWN | SSL_SENT_SHUTDOWN);
    SSL_shutdown(ssl);
    SSL_free(ssl);
    SSL_CTX_free(ctx);
    close(sd);
    return buffer;
}

