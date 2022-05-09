// no-system-llvm
// compile-flags: -O
// ignore-debug: the extra debug assertions can definitely panic

#![crate_type = "lib"]

//! These test that all the assertions and other such checks in the
//! `DefaultHasher` implementation compile out.  The code itself is
//! too complex to bother checking exactly what's emitted, though.

use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;

// CHECK: ; Function Attrs:
// CHECK-SAME: nounwind
// CHECK-NEXT: define void @default_hasher_write_byte(
#[no_mangle]
pub fn default_hasher_write_byte(h: &mut DefaultHasher, b: u8) {
    h.write_u8(b);
}

// CHECK: ; Function Attrs:
// CHECK-SAME: nounwind
// CHECK-NEXT: define void @default_hasher_write_slice(
#[no_mangle]
pub fn default_hasher_write_slice(h: &mut DefaultHasher, bytes: &[u8]) {
    h.write(bytes);
}

// CHECK: ; Function Attrs:
// CHECK-SAME: nounwind
// CHECK-NEXT: define i64 @default_hasher_finish(
#[no_mangle]
pub fn default_hasher_finish(h: &DefaultHasher) -> u64 {
    h.finish()
}
