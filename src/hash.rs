use sha2::{Digest, Sha256};

pub fn compute_hash(content: &str) -> String {
    let mut hasher = Sha256::new(); // no need to crate this with mutex, cause mutex is more expensive than this
    hasher.update(content);
    hex::encode(hasher.finalize())
}
