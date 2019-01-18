// compile-flags: --edition 2018

#![feature(try_blocks)]

pub fn main() {
    let res: Result<u32, i32> = try {
        Err("")?; //~ ERROR the trait bound `i32: std::convert::From<&str>` is not satisfied
        5
    };

    let res: Result<i32, i32> = try {
        "" //~ ERROR type mismatch
    };

    let res: Result<i32, i32> = try { }; //~ ERROR type mismatch

    let res: () = try { }; //~ the trait bound `(): std::ops::TryBlock` is not satisfied

    let res: i32 = try { 5 }; //~ ERROR the trait bound `i32: std::ops::TryBlock` is not satisfied
}
