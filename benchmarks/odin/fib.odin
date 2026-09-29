package main

import "core:fmt"

fibonacci :: proc(n: int) -> int {
    if n <= 0 do return 0
    if n == 1 do return 1
    return fibonacci(n - 1) + fibonacci(n - 2)
}

main :: proc() {
    fmt.println(fibonacci(40))
}
