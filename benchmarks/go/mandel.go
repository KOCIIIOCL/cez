package main

import "fmt"

func mandelbrot() int {
    width := 600
    height := 600
    max_iter := 200
    total := 0
    for py := 0; py < height; py++ {
        y0 := float64(py)/float64(height)*2.0 - 1.0
        for px := 0; px < width; px++ {
            x0 := float64(px)/float64(width)*3.0 - 2.0
            x := 0.0
            y := 0.0
            iter := 0
            for (x*x+y*y <= 4.0) && (iter < max_iter) {
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

func main() {
    fmt.Println(mandelbrot())
}
