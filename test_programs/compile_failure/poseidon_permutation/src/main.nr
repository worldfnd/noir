use std::hash::poseidon2_permutation;

fn main(input1: [u8; 16], input2: [u8; 4], flag: bool) -> pub [u8; 32] {
    let input1 = input1.map(|x| x as Field);
    let input2 = input2.map(|x| x as Field);
    let hash = if flag {
        // Even if on a disabled branch, a call with unexpected input length should cause compile failure.
        poseidon2_permutation(input1)[0]
    } else {
        poseidon2_permutation(input2)[0]
    };
    hash.to_le_bytes()
}
