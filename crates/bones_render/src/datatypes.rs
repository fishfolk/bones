//! Useful data types such as [`Key`].

/// A small ascii byte array stored on the stack and used similarly to a string to represent things
/// like animation keys, etc, without requring a heap allocation.
#[derive(Eq, PartialEq, Copy, Clone, Hash, Debug)]
#[repr(transparent)]
pub struct Key<const N: usize = 24>([u8; N]);

impl<const N: usize> Default for Key<N> {
    fn default() -> Self {
        Self([0; N])
    }
}

impl<'a, const N: usize> TryFrom<&'a str> for Key<N> {
    type Error = KeyError;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

/// An error that may be caused when creating a [`Key`].
#[derive(Copy, Clone, Debug)]
pub enum KeyError {
    /// The key is too long.
    TooLong,
    /// The key is not ASCII.
    NotAscii,
}
impl std::fmt::Display for KeyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyError::TooLong => write!(f, "Key too long."),
            KeyError::NotAscii => write!(f, "Key not ascii."),
        }
    }
}

impl std::error::Error for KeyError {}
impl std::fmt::Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for byte in self.0 {
            write!(f, "{}", byte as char)?;
        }
        Ok(())
    }
}

impl<const N: usize> Key<N> {
    /// Create a [`Key`] from a string.
    ///
    /// # Errors
    ///
    /// Returns an error if the input is too long, or if it is non-ascii.
    pub fn new(s: &str) -> Result<Self, KeyError> {
        if !s.is_ascii() {
            return Err(KeyError::NotAscii);
        }
        if s.len() > N {
            return Err(KeyError::TooLong);
        }
        let mut data = [0; N];
        data[0..s.len()].clone_from_slice(&s.as_bytes()[0..s.len()]);

        Ok(Self(data))
    }
}

#[cfg(feature = "serde")]
mod serde_impl {
    use super::*;
    use serde::{de::Visitor, Deserialize, Serialize};

    impl<'de, const N: usize> Deserialize<'de> for Key<N> {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            deserializer.deserialize_str(KeyVisitor::<N>)
        }
    }

    impl<const N: usize> Serialize for Key<N> {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            let s = String::from_utf8(self.0.to_vec()).unwrap();
            s.serialize(serializer)
        }
    }

    struct KeyVisitor<const N: usize>;
    impl<'de, const N: usize> Visitor<'de> for KeyVisitor<N> {
        type Value = Key<N>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(formatter, "A valid ascii key.")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Key::new(v).map_err(|e| E::custom(e.to_string()))
        }
    }
}
