//! Short prefixed ids for work, deeds, tickets, and atoms.

use rand::RngCore;

pub fn mint(prefix: &str) -> String {
    let mut bytes = [0u8; 5];
    rand::thread_rng().fill_bytes(&mut bytes);
    format!("{prefix}-{}", hex::encode(bytes))
}
