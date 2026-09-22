//! Proven-ancestry journal / markdown append merge (S6 / #96).
//!
//! Both sides must still start with the shared ancestor bytes. Suffixes are
//! concatenated; `<!--ENC-->` blocks inside the ancestor are left intact.

pub fn append_merge(base: &[u8], local: &[u8], remote: &[u8]) -> Option<Vec<u8>> {
    if !local.starts_with(base) || !remote.starts_with(base) {
        return None;
    }
    let local_tail = &local[base.len()..];
    let remote_tail = &remote[base.len()..];
    if local_tail == remote_tail {
        return Some(local.to_vec());
    }
    let mut out = Vec::with_capacity(base.len() + local_tail.len() + remote_tail.len());
    out.extend_from_slice(local);
    out.extend_from_slice(remote_tail);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concatenates_independent_appends_and_keeps_enc_blocks() {
        let enc = b"# claude\n<!--ENC-->gAAAAABshared\n";
        let local = [enc.as_slice(), b"A-only\n"].concat();
        let remote = [enc.as_slice(), b"B-only\n"].concat();
        let merged = append_merge(enc, &local, &remote).unwrap();
        assert!(merged.starts_with(enc));
        assert!(merged
            .windows(b"<!--ENC-->gAAAAABshared".len())
            .any(|w| w == b"<!--ENC-->gAAAAABshared"));
        assert!(merged.windows(6).any(|w| w == b"A-only"));
        assert!(merged.windows(6).any(|w| w == b"B-only"));
    }

    #[test]
    fn rewrite_or_enc_disagreement_is_not_mergeable() {
        let base = b"# claude\n<!--ENC-->gAAAAABold\n";
        let local = b"# claude\n<!--ENC-->gAAAAABnew\nA\n";
        let remote = [base.as_slice(), b"B\n"].concat();
        assert!(append_merge(base, local, &remote).is_none());
        assert!(append_merge(base, b"totally rewritten", remote.as_slice()).is_none());
    }
}
