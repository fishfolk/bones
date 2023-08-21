use std::str::FromStr;

use bones_utils::LabeledId;
use semver::VersionReq;
use serde::Deserialize;

use crate::prelude::*;

/// Deserializeable struct for schema files.
///
/// This struct is required because you can't use `serde(with = "..")` directly on the [`Schema`]
/// enum to make it use nested struct syntax instead of YAML tags for enum representation. Avoiding
/// tags is necessary because we use nested enums such as `vec: primitive: string`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
pub struct SchemaFile {
    /// The schema defined in the file
    #[cfg_attr(
        feature = "serde",
        serde(with = "serde_yaml::with::singleton_map_recursive")
    )]
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub schema: Schema,
}

impl FromStr for AssetPackReq {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some((id, version)) = s.split_once('@') {
            let id = id.parse::<LabeledId>().map_err(|e| e.to_string())?;
            let version = version.parse::<VersionReq>().map_err(|e| e.to_string())?;
            Ok(Self { id, version })
        } else {
            let id = s.parse::<LabeledId>().map_err(|e| e.to_string())?;
            Ok(Self {
                id,
                version: VersionReq::STAR,
            })
        }
    }
}

impl FromStr for SchemaPath {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some((pack_id, schema)) = s.split_once('/') {
            let pack = AssetPackReq::from_str(pack_id)?;

            Ok(SchemaPath {
                pack: Some(pack),
                name: schema.into(),
            })
        } else {
            Ok(SchemaPath {
                pack: None,
                name: s.into(),
            })
        }
    }
}

impl<'de> Deserialize<'de> for SchemaPath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;
        let s = String::deserialize(deserializer)?;
        s.parse::<SchemaPath>().map_err(D::Error::custom)
    }
}
