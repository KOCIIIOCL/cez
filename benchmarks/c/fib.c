#include <stdio.h>
long fibonacci(long n) {
    if (n <= 0) return 0;
    if (n == 1) return 1;
    return fibonacci(n - 1) + fibonacci(n - 2);
}
int main() {
    printf("%ld\n", fibonacci(40));
    return 0;
}
