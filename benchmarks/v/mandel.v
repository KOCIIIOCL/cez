fn mandelbrot() i64 {
    width := 600
    height := 600
    max_iter := 200
    mut total := i64(0)
    for py in 0..height {
        y0 := f64(py) / f64(height) * 2.0 - 1.0
        for px in 0..width {
            x0 := f64(px) / f64(width) * 3.0 - 2.0
            mut x := 0.0
            mut y := 0.0
            mut iter := 0
            for (x*x + y*y <= 4.0) && (iter < max_iter) {
                xtemp := x*x - y*y + x0
                y = 2.0*x*y + y0
                x = xtemp
                iter++
            }
            total += iter
        }
    }
    return total
}

fn main() {
    println(mandelbrot())
}
