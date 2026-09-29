fn mandelbrot() -> i64 {
    let width = 600;
    let height = 600;
    let max_iter = 200;
    let mut total: i64 = 0;
    for py in 0..height {
        let y0 = py as f64 / height as f64 * 2.0 - 1.0;
        for px in 0..width {
            let x0 = px as f64 / width as f64 * 3.0 - 2.0;
            let mut x = 0.0f64;
            let mut y = 0.0f64;
            let mut iter = 0;
            while x * x + y * y <= 4.0 && iter < max_iter {
                let xtemp = x * x - y * y + x0;
                y = 2.0 * x * y + y0;
                x = xtemp;
                iter += 1;
            }
            total += iter as i64;
        }
    }
    total
}
fn main() {
    println!("{}", mandelbrot());
}
