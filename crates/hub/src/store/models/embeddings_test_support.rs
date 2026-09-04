use super::{HubError, VECTOR_DIMENSIONS};
use std::collections::HashMap;
use std::path::Path;

/// Deterministic offline stand-in used only by unit tests. Production always
/// embeds with MiniLM (or the configured API override).
pub fn compute_embedding(text: &str) -> Vec<f32> {
    let mut vector = vec![0.0f32; VECTOR_DIMENSIONS];
    let cleaned = text.trim();
    if cleaned.is_empty() {
        return vector;
    }
    let words: Vec<String> = cleaned
        .split(|c: char| !c.is_alphanumeric() && c != '_' && c != '-')
        .filter(|word| !word.is_empty())
        .map(|word| word.to_ascii_lowercase())
        .collect();
    let mut features: HashMap<String, f32> = HashMap::new();
    for (index, word) in words.iter().enumerate() {
        *features.entry(format!("w:{word}")).or_insert(0.0) += 1.0;
        for width in [3, 4] {
            for chars in word.chars().collect::<Vec<_>>().windows(width) {
                *features
                    .entry(format!("c{width}:{}", chars.iter().collect::<String>()))
                    .or_insert(0.0) += if width == 3 { 0.5 } else { 0.75 };
            }
        }
        if let Some(next) = words.get(index + 1) {
            *features.entry(format!("b:{word}_{next}")).or_insert(0.0) += 1.25;
        }
    }
    for (feature, count) in features {
        let hash = fnv1a_hash(feature.as_bytes());
        let sign = if (hash >> 32) & 1 == 0 { 1.0 } else { -1.0 };
        vector[(hash as usize) % VECTOR_DIMENSIONS] += sign * (1.0 + count.ln()).max(0.1);
    }
    let norm = vector.iter().map(|value| value * value).sum::<f32>().sqrt();
    if norm > 1e-6 {
        for value in &mut vector {
            *value /= norm;
        }
    }
    vector
}

pub(super) fn embed_text(_: &Path, text: &str) -> Result<Vec<f32>, HubError> {
    Ok(compute_embedding(text))
}

fn fnv1a_hash(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325u64, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    })
}
