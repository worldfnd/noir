// `main` takes no `Field`: its inputs are read as bytes, the same on every field.
fn main(x: Field, y: pub u32) {
    assert((x != 0) & (y != 0));
}
