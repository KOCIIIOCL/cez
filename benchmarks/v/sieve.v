fn sieve(limit int) int {
    mut flags := []u8{len: limit + 1, init: 1}
    flags[0] = 0
    flags[1] = 0
    mut p := 2
    for p * p <= limit {
        if flags[p] != 0 {
            mut i := p * p
            for i <= limit {
                flags[i] = 0
                i += p
            }
        }
        p++
    }
    mut count := 0
    for i := 2; i <= limit; i++ {
        if flags[i] != 0 {
            count++
        }
    }
    return count
}

fn main() {
    mut total := 0
    for _ in 0..5 {
        total = sieve(10000000)
    }
    println(total)
}
