// check-pass
// edition:2018

#![allow(dead_code, unused)]

// This pattern was found in the wild in rust-analyzer: https://github.com/rust-analyzer/rust-analyzer/pull/7735

pub fn lookup<T>(slice: &[T], indexes: impl Iterator<Item = usize>) -> Option<Vec<&T>> {
    let values = indexes
        .map(|i| {
            let value = slice.get(i)?;
            Ok(value)
        })
        .collect::<Result<_, _>>()?;
    Some(values)
}

fn main() {
}
