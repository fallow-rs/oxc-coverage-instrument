//! Leaf-name encoding that keeps every emitted path component portable
//! across target filesystems: escaping bytes those filesystems reject,
//! disarming Windows device names, and bounding component length.

use cow_utils::CowUtils as _;

/// Common target filesystems allow 255 bytes or ASCII code units per
/// component. Keep headroom while retaining readable collision suffixes.
pub(super) const MAX_COMPONENT_LEN: usize = 240;

/// One byte past the limit is enough to detect an overlong name without
/// materializing the whole encoding of a pathological input.
const MAX_ENCODED_PREFIX_LEN: usize = MAX_COMPONENT_LEN + 1;

const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// A leaf name that every target filesystem accepts.
#[derive(Clone, Debug)]
pub(super) struct PortableLeaf {
    pub(super) value: String,
    /// `true` when `value` is the caller's preferred name byte for byte:
    /// nothing needed escaping and it fit within [`MAX_COMPONENT_LEN`].
    /// Only a verbatim leaf may claim a name another node also preferred.
    pub(super) is_verbatim: bool,
}

/// Encode `preferred` into a portable leaf name.
///
/// `bad?name.js` -> `bad_x3F_name.js`. Bytes a target filesystem rejects,
/// a trailing dot or space (Windows silently strips both, aliasing two
/// names onto one file), and the leading byte of a Windows device name
/// (`CON`, `LPT1`) are escaped as `_xHH_`. A name still over
/// [`MAX_COMPONENT_LEN`] after encoding is truncated and marked with a
/// hash of the full input so two long names cannot converge.
pub(super) fn portable_leaf(preferred: &str, suffix: &str) -> PortableLeaf {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";

    let disarm_device = is_windows_device_name(preferred);
    let trailing_alias_start = preferred.trim_end_matches(['.', ' ']).len();
    let mut encoded = String::with_capacity(preferred.len().min(MAX_ENCODED_PREFIX_LEN));
    let mut encoded_len = 0usize;
    let mut hash = FNV_OFFSET_BASIS;
    let mut changed = false;
    for (index, byte) in preferred.bytes().enumerate() {
        hash = update_name_hash(hash, byte);
        let escape = !is_portable_ascii_byte(byte)
            || index >= trailing_alias_start
            || (disarm_device && index == 0);
        if escape {
            let escaped =
                [b'_', b'x', HEX[usize::from(byte >> 4)], HEX[usize::from(byte & 0x0f)], b'_'];
            push_encoded_prefix(&mut encoded, &escaped);
            encoded_len = encoded_len.saturating_add(escaped.len());
            changed = true;
        } else {
            push_encoded_prefix(&mut encoded, &[byte]);
            encoded_len = encoded_len.saturating_add(1);
        }
    }

    let value = if encoded_len <= MAX_COMPONENT_LEN {
        encoded
    } else {
        bound_component(&encoded, suffix, hash)
    };
    PortableLeaf { is_verbatim: !changed && encoded_len <= MAX_COMPONENT_LEN, value }
}

fn push_encoded_prefix(encoded: &mut String, bytes: &[u8]) {
    let remaining = MAX_ENCODED_PREFIX_LEN.saturating_sub(encoded.len());
    encoded.extend(bytes.iter().take(remaining).map(|byte| char::from(*byte)));
}

/// Nth alternative for a leaf whose preferred name is taken.
///
/// `a.js.html` -> `a.js.oxc-file-1.html`. The discriminator goes before
/// `suffix` so the page keeps its `.html` extension.
pub(super) fn collision_leaf(preferred: &str, kind: &str, suffix: &str, attempt: u32) -> String {
    let stem = preferred.strip_suffix(suffix).unwrap_or(preferred);
    let discriminator = format!(".oxc-{kind}-{attempt}");
    let candidate = format!("{stem}{discriminator}{suffix}");
    if candidate.len() <= MAX_COMPONENT_LEN {
        return candidate;
    }

    let hash = stable_name_hash(preferred.as_bytes());
    let marker = format!("_h{hash:016x}");
    let prefix_len = MAX_COMPONENT_LEN - marker.len() - discriminator.len() - suffix.len();
    format!("{}{}{}{}", &stem[..stem.len().min(prefix_len)], marker, discriminator, suffix)
}

fn bound_component(value: &str, suffix: &str, hash: u64) -> String {
    if value.len() <= MAX_COMPONENT_LEN {
        return value.to_owned();
    }

    let stem = value.strip_suffix(suffix).unwrap_or(value);
    let marker = format!("_h{hash:016x}");
    let prefix_len = MAX_COMPONENT_LEN - marker.len() - suffix.len();
    format!("{}{}{}", &stem[..stem.len().min(prefix_len)], marker, suffix)
}

fn stable_name_hash(bytes: &[u8]) -> u64 {
    bytes.iter().fold(FNV_OFFSET_BASIS, |hash, byte| update_name_hash(hash, *byte))
}

fn update_name_hash(hash: u64, byte: u8) -> u64 {
    (hash ^ u64::from(byte)).wrapping_mul(FNV_PRIME)
}

/// Printable ASCII minus the characters Windows forbids in a filename.
pub(super) fn is_portable_ascii_byte(byte: u8) -> bool {
    (b' '..=b'~').contains(&byte) && !b"<>:\"/\\|?*".contains(&byte)
}

/// Windows resolves these stems to a device rather than a file, whatever
/// extension or trailing space follows: opening `CON.html` for writing
/// talks to the console.
pub(super) fn is_windows_device_name(value: &str) -> bool {
    let stem = value.split('.').next().unwrap_or(value).trim_end_matches(' ');
    let stem = stem.cow_to_ascii_lowercase();
    matches!(stem.as_ref(), "con" | "prn" | "aux" | "nul")
        || stem
            .strip_prefix("com")
            .or_else(|| stem.strip_prefix("lpt"))
            .is_some_and(|digit| digit.len() == 1 && matches!(digit.as_bytes()[0], b'1'..=b'9'))
}
