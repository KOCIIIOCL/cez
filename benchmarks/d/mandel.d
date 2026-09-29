import std.stdio;

long mandelbrot() {
    int width = 600;
    int height = 600;
    int max_iter = 200;
    long total = 0;
    for (int py = 0; py < height; py++) {
        double y0 = cast(double)py / cast(double)height * 2.0 - 1.0;
        for (int px = 0; px < width; px++) {
            double x0 = cast(double)px / cast(double)width * 3.0 - 2.0;
            double x = 0.0;
            double y = 0.0;
            int iter = 0;
            while ((x * x + y * y <= 4.0) && (iter < max_iter)) {
                double xtemp = x * x - y * y + x0;
                y = 2.0 * x * y + y0;
                x = xtemp;
                iter++;
            }
            total += iter;
        }
    }
    return total;
}

void main() {
    writeln(mandelbrot());
}
