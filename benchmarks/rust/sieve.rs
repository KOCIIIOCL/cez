fn sieve(limit: usize) -> usize {
    let mut flags = vec![1u8; limit + 1];
    flags[0] = 0;
    flags[1] = 0;
    let mut p = 2;
    while p * p <= limit {
        if flags[p] != 0 {
            let mut i = p * p;
            while i <= limit {
                flags[i] = 0;
                i += p;
            }
        }
        p += 1;
    }
    let mut count = 0;
    for i in 2..=limit {
        if flags[i] != 0 { count += 1; }
    }
    count
}
fn main() {
    let mut total = 0;
    for _ in 0..5 {
        total = sieve(10_000_000);
    }
    println!("{}", total);
}
