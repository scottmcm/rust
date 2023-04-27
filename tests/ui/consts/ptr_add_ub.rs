#![feature(strict_provenance)]

// normalize-stderr-test "alloc\d+" -> "allocN"
// normalize-stderr-test "2147483647" -> "0x7F..FF"
// normalize-stderr-test "9223372036854775807" -> "0x7F..FF"
// normalize-stderr-test "0x80{4,}" -> "0x80..00"
// normalize-stderr-test "0xFF{4,}" -> "0xFF..FF"

// For things that don't wrap, give the usual precise error.
pub const ADD_ISIZE_MAX: *const u8 = unsafe { [0_u8; 123].as_ptr().add(isize::MAX as usize) };

// Report adding a value that's too large to ever pass.
pub const ADD_ISIZE_MAX_PLUS_ONE: *const u8 = unsafe { [0_u8; 123].as_ptr().add(isize::MAX as usize + 1) };

// Make sure `add` isn't just using `as isize`, which hides this problem.
pub const ADD_DOES_NOT_WRAP: char = unsafe {
    let a = ['R', 'u', 's', 't'];
    let p = a.as_ptr();
    *p.add(1).add(usize::MAX)
};

// ZST pointees can be offset all over the place no problem.
pub const ADD_ZST_USIZE_MAX: *const () = unsafe { std::ptr::invalid::<()>(1).add(usize::MAX) };

fn main() {}
