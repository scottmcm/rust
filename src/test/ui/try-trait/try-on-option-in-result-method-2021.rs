// edition:2021

#![allow(dead_code, unused)]

///! This worked in 2018, but for 2021 we get to fix that.

struct Tricky;

impl<T: std::fmt::Debug> From<T> for Tricky {
    fn from(_: T) -> Tricky { Tricky }
}

fn foo() -> Result<(), Tricky> {
    None?; //~ ERROR the `?` operator can only be used in a function that returns `Result` or `Option`
    Ok(())
}

fn main() {
    assert!(matches!(foo(), Err(Tricky)));
}
