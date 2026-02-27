use sha2::{Digest, Sha256};

pub fn compute_hash(content: &str) -> String {
    let mut hasher = Sha256::new(); // no need to crate this with mutex, cause mutex is more expensive than this
    hasher.update(content);
    hex::encode(hasher.finalize())
}

mod hash_tests {
    use super::*;

    #[test]
    fn test_hello_world_hash() {
        let content = "Hello, world!";
        let hash = compute_hash(content);
        let hello_world_hash = "315f5bdb76d078c43b8ac0064e4a0164612b1fce77c869345bfc94c75894edd3";
        assert_eq!(hash, hello_world_hash);
    }
}
