fn fibonacci(n i64) i64 {
    if n <= 0 { return 0 }
    if n == 1 { return 1 }
    return fibonacci(n - 1) + fibonacci(n - 2)
}

fn main() {
    println(fibonacci(40))
}
