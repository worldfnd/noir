fn main() {
    let x: Field = std::hint::black_box(1);
    let y: Field = std::hint::black_box(2);
    let flag = (x == 1) | (y == 2);
    assert(flag | false == flag);
}
