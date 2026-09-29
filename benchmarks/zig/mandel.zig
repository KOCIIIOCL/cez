const std = @import("std");

fn mandelbrot() i64 {
    const width: usize = 600;
    const height: usize = 600;
    const max_iter: usize = 200;
    var total: i64 = 0;
    var py: usize = 0;
    while (py < height) : (py += 1) {
        const y0 = @as(f64, @floatFromInt(py)) / @as(f64, @floatFromInt(height)) * 2.0 - 1.0;
        var px: usize = 0;
        while (px < width) : (px += 1) {
            const x0 = @as(f64, @floatFromInt(px)) / @as(f64, @floatFromInt(width)) * 3.0 - 2.0;
            var x: f64 = 0.0;
            var y: f64 = 0.0;
            var iter: usize = 0;
            while ((x * x + y * y <= 4.0) and (iter < max_iter)) : (iter += 1) {
                const xtemp = x * x - y * y + x0;
                y = 2.0 * x * y + y0;
                x = xtemp;
            }
            total += @as(i64, @intCast(iter));
        }
    }
    return total;
}

pub fn main() !void {
    const stdout = std.io.getStdOut().writer();
    try stdout.print("{d}\n", .{mandelbrot()});
}
