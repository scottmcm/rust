// run-pass

fn main() {
    for x in [1, 2][..].into_iter() {
        let _: &i32 = x;
    }
}
