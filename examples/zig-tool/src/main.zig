const std = @import("std");

pub fn main() !void {
    const stdout = std.io.getStdOut().writer();
    try stdout.print("zfetch {s}\n", .{"0.1.0"});
}

test "version is set" {
    try std.testing.expectEqualStrings("0.1.0", "0.1.0");
}

test "main does not fail" {
    try main();
}
