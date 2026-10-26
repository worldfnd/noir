// Test that type aliases to primitive types can use path-syntax method calls.
type MyField = Field;

// Chained alias
type AliasOfAlias = MyField;

fn main(x: u8) {
    let expected: [bool; 8] = (x as Field).to_le_bits();

    // Direct method on Field alias gives same result as dot syntax
    let bits: [bool; 8] = MyField::to_le_bits(x as Field);
    assert(bits == expected);

    // Chained alias gives same result as dot syntax
    let bits2: [bool; 8] = AliasOfAlias::to_le_bits(x as Field);
    assert(bits2 == expected);
}
