use std::fmt;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::{CoreError, Result};

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Fingerprint(String);

impl Fingerprint {
    pub fn parse(input: impl AsRef<str>) -> Result<Self> {
        let raw = input.as_ref().trim();
        let Some(hex_part) = raw.strip_prefix("sha256:") else {
            return Err(CoreError::InvalidFingerprint(raw.to_owned()));
        };

        if hex_part.len() != 64 || !hex_part.chars().all(|ch| ch.is_ascii_hexdigit()) {
            return Err(CoreError::InvalidFingerprint(raw.to_owned()));
        }

        Ok(Self(format!("sha256:{}", hex_part.to_ascii_lowercase())))
    }

    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let digest = Sha256::digest(bytes);
        Self(format!("sha256:{}", hex::encode(digest)))
    }

    pub fn from_json<T: Serialize>(value: &T) -> Result<Self> {
        let encoded = serde_json::to_vec(value)?;
        Ok(Self::from_bytes(&encoded))
    }

    pub fn from_reader<R: Read>(mut reader: R) -> io::Result<Self> {
        let mut hasher = Sha256::new();
        let mut buffer = [0_u8; 16 * 1024];

        loop {
            let read = reader.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
        }

        Ok(Self(format!("sha256:{}", hex::encode(hasher.finalize()))))
    }

    pub fn from_file(path: &Path) -> io::Result<Self> {
        let file = File::open(path)?;
        Self::from_reader(file)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn algorithm(&self) -> &str {
        "sha256"
    }

    #[must_use]
    pub fn hex(&self) -> &str {
        &self.0[7..]
    }

    #[must_use]
    pub fn short(&self) -> &str {
        &self.hex()[..12]
    }
}

impl fmt::Display for Fingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for Fingerprint {
    type Error = CoreError;

    fn try_from(value: String) -> Result<Self> {
        Self::parse(value)
    }
}

impl From<Fingerprint> for String {
    fn from(value: Fingerprint) -> Self {
        value.0
    }
}

#[cfg(test)]
mod tests {
    use super::Fingerprint;

    #[test]
    fn normalizes_valid_fingerprint() {
        let parsed = Fingerprint::parse(
            "sha256:ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789",
        )
        .expect("fingerprint should parse");

        assert_eq!(
            parsed.as_str(),
            "sha256:abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789"
        );
    }

    #[test]
    fn hashes_bytes_stably() {
        let first = Fingerprint::from_bytes(b"codex1");
        let second = Fingerprint::from_bytes(b"codex1");
        let third = Fingerprint::from_bytes(b"codex1-core");

        assert_eq!(first, second);
        assert_ne!(first, third);
        assert_eq!(first.algorithm(), "sha256");
        assert_eq!(first.short().len(), 12);
    }
}
