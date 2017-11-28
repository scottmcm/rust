// Copyright 2017 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#[inline(never)]
fn nop<T>(_: T) {}

fn test_simple(x: [u32; 1]) -> u32 {
    x[0]
}

fn test_after_branches(b: bool) -> u32 {
    let x = 1;
    if b { nop(2) }
    else { nop(3) }
    x
}

// Ensure this case *doesn't* get accidentally optimized
#[allow(unused_assignments)]
fn test_diffent_values(b: bool) -> u32 {
    let mut x = 1;
    if b { x = 2; }
    else { x = 3; }
    x
}

fn test_reused(x: [u32; 2]) -> u32 {
    let mut i = 0;
    let a = x[i];
    i = 1;
    let b = x[i];
    a + b
}

fn test_borrowed() -> u32 {
    let mut i = 0;
    nop(&mut i);
    i
}

fn main() {
    // Make sure the functions actually get instantiated.
    test_simple([0]);
    test_after_branches(true);
    test_diffent_values(true);
    test_reused([1, 2]);
    test_borrowed();
}

// END RUST SOURCE

// START rustc.test_simple.ConstPropagation.before.mir
//     _3 = const 0usize;
//     _4 = const 1usize;
//     _5 = Lt(_3, _4);
// END rustc.test_simple.ConstPropagation.before.mir
// START rustc.test_simple.ConstPropagation.after.mir
//     _5 = Lt(const 0usize, const 1usize);
// END rustc.test_simple.ConstPropagation.after.mir

// START rustc.test_after_branches.ConstPropagation.before.mir
//     _2 = const 1u32;
//     ...
//     _5 = _2;
//     _0 = move _5;
// END rustc.test_after_branches.ConstPropagation.before.mir
// START rustc.test_after_branches.ConstPropagation.after.mir
//     _5 = const 1u32;
//     _0 = move _5;
// END rustc.test_after_branches.ConstPropagation.after.mir

// START rustc.test_diffent_values.ConstPropagation.before.mir
//     _2 = const 1u32;
//     ...
//     _2 = const 2u32;
//     ...
//     _2 = const 3u32;
//     ...
//     _5 = _2;
//     _0 = move _5;
// END rustc.test_diffent_values.ConstPropagation.before.mir
// START rustc.test_diffent_values.ConstPropagation.after.mir
//     _5 = _2;
//     _0 = move _5;
// END rustc.test_diffent_values.ConstPropagation.after.mir

// START rustc.test_reused.ConstPropagation.before.mir
//     _2 = const 0usize;
//     ...
//     _5 = _2;
//     _6 = const 2usize;
//     _7 = Lt(_5, _6);
//     ...
//     _2 = const 1usize;
//     ...
//     _10 = _2;
//     _11 = const 2usize;
//     _12 = Lt(_10, _11);
// END rustc.test_reused.ConstPropagation.before.mir
// START rustc.test_reused.ConstPropagation.after.mir
//     _7 = Lt(const 0usize, const 2usize);
//     ...
//     _12 = Lt(const 1usize, const 2usize);
// END rustc.test_reused.ConstPropagation.after.mir

// START rustc.test_borrowed.ConstPropagation.before.mir
//     _1 = const 0u32;
//     ...
//     _3 = &mut _1;
//     ...
//     _4 = _1;
//     _0 = move _4;
// END rustc.test_borrowed.ConstPropagation.before.mir
// START rustc.test_borrowed.ConstPropagation.after.mir
//     _1 = const 0u32;
//     ...
//     _3 = &mut _1;
//     ...
//     _4 = _1;
//     _0 = move _4;
// END rustc.test_borrowed.ConstPropagation.after.mir
