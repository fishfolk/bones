use crate::prelude::*;

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

impl FromStr for SchemaId {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some((pack_id, schema)) = s.split_once('/') {
            let pack = AssetPackReq::from_str(pack_id)?;

            Ok(SchemaId {
                pack: Some(pack),
                name: schema.into(),
            })
        } else {
            Ok(SchemaId {
                pack: None,
                name: s.into(),
            })
        }
    }
}

impl<'de> Deserialize<'de> for SchemaId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;
        let s = String::deserialize(deserializer)?;
        s.parse::<SchemaId>().map_err(D::Error::custom)
    }
}
