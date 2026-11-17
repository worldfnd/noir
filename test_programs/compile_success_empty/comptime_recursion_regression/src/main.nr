fn main(x: u64, y: u64) {
    let x = Field::from(x);
    let y = Field::from(y);
    let flag = (x == 1) | (y == 2);
    assert(flag | false == flag);
}
