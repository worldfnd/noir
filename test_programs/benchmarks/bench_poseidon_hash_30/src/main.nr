global SIZE: u32 = 30;

fn main(input: [[u8; 2]; SIZE]) -> pub [[u8; 32]; SIZE] {
    let mut results: [[u8; 32]; SIZE] = [[0; 32]; SIZE];
    for i in 0..SIZE {
        results[i] = poseidon::poseidon::bn254::hash_2(input[i].map(|x| x as Field)).to_le_bytes();
    }

    results
}
