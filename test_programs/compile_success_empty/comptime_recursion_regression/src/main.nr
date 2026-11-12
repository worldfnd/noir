fn main(x: u64, y: u64) {
    let x = x as Field;
    let y = y as Field;
    let flag = (x == 1) | (y == 2);
    assert(flag | false == flag);
}
