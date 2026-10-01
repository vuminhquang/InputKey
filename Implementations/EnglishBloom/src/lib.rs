use inputkey_core_abstractions::LexiconPort;

const BLOOM_BITS: u32 = 2_097_152;
const BLOOM_K: u32 = 12;
const BLOOM: &[u8] = include_bytes!("../english.bloom");

#[derive(Clone, Copy, Debug, Default)]
pub struct EnglishBloom;

impl EnglishBloom {
    pub const fn new() -> Self {
        Self
    }
}

fn fnv1a(text: &[u8], seed: u32) -> u32 {
    let mut hash = seed;
    for byte in text {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

impl LexiconPort for EnglishBloom {
    fn has_word(&self, word: &str) -> bool {
        let lower = word.to_ascii_lowercase();
        let bytes = lower.as_bytes();
        if !(2..=32).contains(&bytes.len()) || BLOOM.is_empty() {
            return false;
        }
        if !bytes.iter().all(u8::is_ascii_lowercase) {
            return false;
        }

        let h1 = fnv1a(bytes, 0x811c_9dc5);
        let h2 = fnv1a(bytes, 0x9e37_79b9) | 1;
        for j in 0..BLOOM_K {
            let index = h1
                .wrapping_add(j.wrapping_mul(h2))
                .wrapping_add(j.wrapping_mul(j))
                & (BLOOM_BITS - 1);
            if BLOOM[(index >> 3) as usize] & (1 << (index & 7)) == 0 {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_words_and_rejections() {
        let lexicon = EnglishBloom::new();
        assert!(lexicon.has_word("password"));
        assert!(lexicon.has_word("software"));
        assert!(!lexicon.has_word("rể"));
        assert!(!lexicon.has_word("a"));
    }
}
