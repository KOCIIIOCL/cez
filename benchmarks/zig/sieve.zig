const std = @import("std");

fn sieve(allocator: std.mem.Allocator, limit: usize) !usize {
    const flags = try allocator.alloc(u8, limit + 1);
    defer allocator.free(flags);
    @memset(flags, 1);
    flags[0] = 0;
    flags[1] = 0;
    var p: usize = 2;
    while (p * p <= limit) : (p += 1) {
        if (flags[p] != 0) {
            var i = p * p;
            while (i <= limit) : (i += p) {
                flags[i] = 0;
            }
        }
    }
    var count: usize = 0;
    for (flags[2..]) |val| {
        if (val != 0) count += 1;
    }
    return count;
}

pub fn main() !void {
    const stdout = std.io.getStdOut().writer();
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();
    var total: usize = 0;
    var iter: usize = 0;
    while (iter < 5) : (iter += 1) {
        total = try sieve(allocator, 10000000);
    }
    try stdout.print("{d}\n", .{total});
}
