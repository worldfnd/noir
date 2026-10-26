global EXPONENTIATE: [[Field; 2]; 2] = [[1, 1], [0, 0]];

fn main(x: u8, y: pub u8) {
    let mut acc: Field = 0;
    for i in 0..2 {
        for j in 0..2 {
            acc += EXPONENTIATE[i][j];
        }
    }
    assert(!acc.lt(x as Field));
    assert(x != y);

    dummy_again(x as Field, y as Field);
}

fn dummy_again(x: Field, y: Field) {
    let mut acc: Field = 0;
    for i in 0..2 {
        for j in 0..2 {
            acc += EXPONENTIATE[i][j];
        }
    }
    assert(x != y);
    assert(!acc.lt(x));
}
