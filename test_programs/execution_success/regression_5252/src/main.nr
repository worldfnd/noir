use poseidon;

global NUM_HASHES: u32 = 3;
global HASH_LENGTH: u32 = 20;

pub fn poseidon_hash<let N: u32>(inputs: [Field; N]) -> Field {
    poseidon::poseidon2::Poseidon2::hash(inputs, inputs.len())
}

fn main(
    to_hash: [[u64; HASH_LENGTH]; NUM_HASHES],
    enable: [bool; NUM_HASHES],
) -> pub [[u8; 32]; NUM_HASHES] {
    let to_hash = to_hash.map(|row| row.map(|x| x as Field));
    let mut result = [0; NUM_HASHES];
    for i in 0..NUM_HASHES {
        let enable = enable[i];
        let to_hash = to_hash[i];
        if enable {
            result[i] = poseidon_hash(to_hash) + poseidon::poseidon::bn254::sponge(to_hash);
        }
    }
    result.map(|hash: Field| hash.to_le_bytes())
}
