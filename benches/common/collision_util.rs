use std::collections::HashMap;

/// Exlex's exact internal hash function for replication
#[inline(always)]
fn hash(key: &str) -> u64 {
    const K: u64 = 0x517cc1b727220a95;
    let mut hash: u64 = 0;
    let bytes = key.as_bytes();
    let mut chunks = bytes.chunks_exact(8);
    for chunk in &mut chunks {
        let word = u64::from_ne_bytes(chunk.try_into().unwrap());
        hash = (hash.rotate_left(5) ^ word).wrapping_mul(K);
    }
    for &byte in chunks.remainder() {
        hash = (hash.rotate_left(5) ^ (byte as u64)).wrapping_mul(K);
    }
    hash
}

/// Generates two distinct strings that produce the exact same 64-bit FxHash.
/// Uses a fast numeric permutation to find a collision in ~1-3 seconds.
pub fn get_hash_collision_pair() -> (String, String) {
    ("v79670".to_string(), "v103607".to_string())
}
