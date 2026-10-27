fn main() -> pub u8 {
    let mut _: &mut [&mut &mut u16; 1] = &mut [&mut &mut 1_u16];
    let r: [u8] = @[];
    let i: u32 = 1;
    r[i]
}
