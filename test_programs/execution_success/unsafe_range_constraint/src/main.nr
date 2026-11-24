// Test that we can apply a range constraint to a field using
// a builtin.
fn main(x: u64) {
    (Field::from(x)).assert_max_bit_size::<48>();
}
