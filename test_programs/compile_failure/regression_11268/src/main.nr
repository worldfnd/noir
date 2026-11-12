fn main(c: bool) -> pub u8 {
    let f = || &mut 1;
    let g = || &mut 2;
    let h = if c { f } else { g };
    let x = h();
    *x
}
