//! Path-derived coverage binding and stale-cache hashes.

/// Stable DJB31 hex hash. Used for both the per-file coverage function name
/// and Istanbul's stale-cache guard hash on the embedded coverage object.
pub fn djb31_hex(input: &str) -> String {
    let mut hash: u64 = 0;
    for byte in input.bytes() {
        hash = hash.wrapping_mul(31).wrapping_add(u64::from(byte));
    }
    format!("{hash:x}")
}

/// Generate a deterministic coverage function name from the file path.
pub fn generate_cov_fn_name(file_path: &str) -> String {
    format!("cov_{}", djb31_hex(file_path))
}
