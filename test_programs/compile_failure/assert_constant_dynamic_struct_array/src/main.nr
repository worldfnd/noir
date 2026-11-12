struct Foo {
    field: Field,
    array: [Field; 3],
    vector: [Field],
}

fn main(
    dynamic_one: u8, // == 1
) {
    let foo_dynamic_array = Foo { field: 0, array: [dynamic_one as Field, 2, 3], vector: [].as_vector() };
    assert_constant(foo_dynamic_array);
}
