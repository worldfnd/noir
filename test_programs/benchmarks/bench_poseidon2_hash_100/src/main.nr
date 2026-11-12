global SIZE: u32 = 100;

fn main(input: [[u8; 2]; SIZE]) -> pub [[u8; 32]; SIZE] {
    let mut results: [[u8; 32]; SIZE] = [[0; 32]; SIZE];
    for i in 0..SIZE {
        results[i] =
            poseidon::poseidon2::Poseidon2::hash(input[i].map(|x| x as Field), 2).to_le_bytes();
    }

    results
}
