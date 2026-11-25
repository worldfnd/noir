use std::convert::UnsafeFrom;
// The element has two fields but only one of them occupies an ACIR cell.
struct S {
    a: [Field; 0],
    b: Field,
}

fn main(i: u32, x: u8) -> pub u8 {
    let x = Field::from(x);
    let arr = [S { a: [], b: x }, S { a: [], b: x + 1 }, S { a: [], b: x + 2 }, S { a: [], b: x + 3 }];
    u8::unsafe_from(arr[i].b)
}
