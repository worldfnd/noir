fn main(a: pub u32) -> pub u8 {
    if a == 0 {
        let c: [&mut u8] = [(&mut 1)].as_vector();
        1 / { (*c[3]) }
    } else {
        2
    }
}
