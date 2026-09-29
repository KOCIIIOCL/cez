#include <stdio.h>
#include <stdlib.h>
#include <string.h>

long sieve(long limit) {
    unsigned char *flags = (unsigned char *)malloc(limit + 1);
    if (!flags) return 0;
    memset(flags, 1, limit + 1);
    flags[0] = 0;
    flags[1] = 0;
    for (long p = 2; p * p <= limit; p++) {
        if (flags[p]) {
            for (long i = p * p; i <= limit; i += p) {
                flags[i] = 0;
            }
        }
    }
    long count = 0;
    for (long i = 2; i <= limit; i++) {
        if (flags[i]) count++;
    }
    free(flags);
    return count;
}

int main() {
    long total = 0;
    for (int iter = 0; iter < 5; iter++) {
        total = sieve(10000000);
    }
    printf("%ld\n", total);
    return 0;
}
