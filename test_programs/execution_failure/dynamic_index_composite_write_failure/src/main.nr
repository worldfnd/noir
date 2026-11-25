fn main(i: u32, mut a: [(u8, bool); 3]) -> pub u8 {
    a[i] = (7, true);
    a[0].0
}
