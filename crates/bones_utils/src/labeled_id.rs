use std::str::FromStr;

use ulid::Ulid;

/// A [`Ulid`] with a human-readable ascii prefix.
///
/// This is essentially like a [TypeId](https://github.com/jetpack-io/typeid), but the prefix can be
/// any ascii string instead of only ascii lowercase.
#[derive(Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct LabeledId {
    /// The prefix
    prefix: Option<[u8; 63]>,
    /// The ULID.
    ulid: Ulid,
}

impl std::fmt::Debug for LabeledId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "LabeledId({self})")
    }
}

/// Error creating a [`LabledId`].
#[derive(Debug)]
pub enum LabeledIdCreateError {
    /// The prefix was too long ( greater than 63 chars ).
    PrefixTooLong,
    /// The prefix was not ASCII.
    PrefixNotAscii,
}

impl std::error::Error for LabeledIdCreateError {}
impl std::fmt::Display for LabeledIdCreateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LabeledIdCreateError::PrefixTooLong => write!(
                f,
                "Labled ID prefix is too long ( maxumum length is 63 chars )."
            ),
            LabeledIdCreateError::PrefixNotAscii => write!(f, "Labeled ID prefix is not ASCII"),
        }
    }
}

impl LabeledId {
    /// Create a new labeled ID with the given prefix.
    pub fn new(prefix: &str) -> Result<Self, LabeledIdCreateError> {
        Self::new_with_ulid(prefix, Ulid::new())
    }

    /// Create a new labeled ID with the given prefix and ULID.
    pub fn new_with_ulid(prefix: &str, ulid: Ulid) -> Result<Self, LabeledIdCreateError> {
        if prefix.is_empty() {
            Ok(Self { prefix: None, ulid })
        } else if prefix.len() > 63 {
            Err(LabeledIdCreateError::PrefixTooLong)
        } else if !prefix.is_ascii() {
            Err(LabeledIdCreateError::PrefixNotAscii)
        } else {
            let mut prefix_bytes = [0; 63];
            prefix_bytes[0..prefix.len()].copy_from_slice(prefix.as_bytes());

            Ok(Self {
                prefix: Some(prefix_bytes),
                ulid,
            })
        }
    }

    /// Get the prefix of the ID.
    pub fn prefix(&self) -> &str {
        self.prefix
            .as_ref()
            .map(|x| {
                let prefix_len = Self::prefix_len(x);
                let bytes = &x[0..prefix_len];
                std::str::from_utf8(bytes).unwrap()
            })
            .unwrap_or("")
    }

    /// Get the [`Ulid`] of the ID.
    pub fn ulid(&self) -> Ulid {
        self.ulid
    }

    fn prefix_len(prefix: &[u8; 63]) -> usize {
        let mut len = 0;
        while prefix[len] != 0 {
            len += 1;
        }
        len
    }
}

impl std::fmt::Display for LabeledId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(prefix) = &self.prefix {
            if !prefix.is_ascii() {
                return Err(std::fmt::Error);
            }
            let prefix_len = Self::prefix_len(prefix);
            write!(
                f,
                "{}_{}",
                String::from_utf8(prefix[0..prefix_len].into()).unwrap(),
                self.ulid
            )
        } else {
            write!(f, "{}", self.ulid)
        }
    }
}

/// Errors that can happen while parsing a [`LabeledId`].
#[derive(Debug)]
pub enum LabledIdParseError {
    /// The ID is in the wrong format.
    InvalidFormat,
    /// The ULID could not be parsed.
    UlidDecode(ulid::DecodeError),
    /// Error creating ID
    CreateError(LabeledIdCreateError),
}

impl std::fmt::Display for LabledIdParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LabledIdParseError::InvalidFormat => {
                write!(f, "The Labeled ID is in the wrong format.")
            }
            LabledIdParseError::UlidDecode(e) => write!(f, "Error decoding ULID: {e}"),
            LabledIdParseError::CreateError(e) => write!(f, "Error creating LabeledId: {e}"),
        }
    }
}

impl FromStr for LabeledId {
    type Err = LabledIdParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use LabledIdParseError::*;
        if let Some((prefix, ulid_text)) = s.rsplit_once('_') {
            let ulid = Ulid::from_str(ulid_text).map_err(UlidDecode)?;
            LabeledId::new_with_ulid(prefix, ulid).map_err(CreateError)
        } else {
            let ulid = Ulid::from_str(s).map_err(UlidDecode)?;
            Ok(LabeledId { prefix: None, ulid })
        }
    }
}

#[cfg(feature = "serde")]
mod ser_de {
    use super::*;
    use serde::{Deserialize, Serialize};

    impl Serialize for LabeledId {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            serializer.serialize_str(&self.to_string())
        }
    }

    impl<'de> Deserialize<'de> for LabeledId {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            use serde::de::Error;
            let s = String::deserialize(deserializer)?;
            s.parse().map_err(|e| D::Error::custom(format!("{e}")))
        }
    }
}

#[cfg(test)]
mod test {
    use crate::LabeledId;

    #[test]
    fn smoke() {
        let id = LabeledId::new("asset").unwrap();
        let parsed: LabeledId = id.to_string().parse().unwrap();

        assert_eq!(id, parsed)
    }
}
