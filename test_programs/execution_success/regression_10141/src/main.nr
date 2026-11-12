fn main(a: u32) -> pub u16 {
    foo()[a].1
}

fn foo() -> [([u64; 0], u16); 1] {
    [([], 10)]
}
