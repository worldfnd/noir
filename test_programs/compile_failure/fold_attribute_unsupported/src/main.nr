fn main(x: u32) -> pub u32 {
    double(x)
}

#[fold]
fn double(x: u32) -> u32 {
    x * 2
}
