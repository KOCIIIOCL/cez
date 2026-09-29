package main

import "core:fmt"
import "core:mem"

sieve :: proc(limit: int) -> int {
    flags := make([]u8, limit + 1)
    defer delete(flags)
    mem.set(&flags[0], 1, limit + 1)
    flags[0] = 0
    flags[1] = 0
    for p := 2; p * p <= limit; p += 1 {
        if flags[p] != 0 {
            for i := p * p; i <= limit; i += p {
                flags[i] = 0
            }
        }
    }
    count := 0
    for i := 2; i <= limit; i += 1 {
        if flags[i] != 0 do count += 1
    }
    return count
}

main :: proc() {
    total := 0
    for _ in 0..<5 {
        total = sieve(10_000_000)
    }
    fmt.println(total)
}
