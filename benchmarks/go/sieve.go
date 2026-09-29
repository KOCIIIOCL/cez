package main

import "fmt"

func sieve(limit int) int {
    flags := make([]byte, limit+1)
    for i := range flags {
        flags[i] = 1
    }
    flags[0] = 0
    flags[1] = 0
    for p := 2; p*p <= limit; p++ {
        if flags[p] != 0 {
            for i := p * p; i <= limit; i += p {
                flags[i] = 0
            }
        }
    }
    count := 0
    for i := 2; i <= limit; i++ {
        if flags[i] != 0 {
            count++
        }
    }
    return count
}

func main() {
    total := 0
    for iter := 0; iter < 5; iter++ {
        total = sieve(10000000)
    }
    fmt.Println(total)
}
