fn main(x: Field) -> pub Field {
    double(x)
}

#[fold]
fn double(x: Field) -> Field {
    x * 2
}
