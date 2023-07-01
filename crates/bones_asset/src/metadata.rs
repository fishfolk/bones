use std::{cell::RefCell, collections::HashMap};

use crate::prelude::*;

/// Cell that wraps [`AssetLoadCtx`] used when deserializing metadata.
pub struct AssetLoadCtxCell<'a, Io: AssetIo>(RefCell<AssetLoadCtx<'a, Io>>);

impl<'a, Io: AssetIo> AssetLoadCtxCell<'a, Io> {
    /// Create a new context cell.
    pub fn new(
        server: &'a mut AssetServer,
        io: &'a Io,
        pack: Option<&'a str>,
        path: &'a Path,
    ) -> Self {
        Self(RefCell::new(AssetLoadCtx::new(server, io, pack, path)))
    }

    /// Get the inner [`AssetLoadCtx`].
    pub fn into_inner(self) -> AssetLoadCtx<'a, Io> {
        self.0.into_inner()
    }
}

/// The context used when loading assets.
pub struct AssetLoadCtx<'a, Io: AssetIo> {
    /// The asset server.
    pub server: &'a mut AssetServer,
    /// The [`AssetIo`] implementation.
    pub io: &'a Io,
    /// The path to the asset being loaded.
    pub path: &'a Path,
    /// The asset pack that this asset is being loaded from, or [`None`] if it is from the default
    /// pack.
    pub pack: Option<&'a str>,
    /// The [`Cid`]s of assets depended on by this asset.
    pub dependencies: Vec<Cid>,
    /// The [`Cid`] of this asset.
    pub cid: Cid,
    //// The runtime ID of this asset. This ID is the one stored in [`Handle<T>`]s and
    ///[`UntypedHandle`]s.
    ///
    /// This ID remains the same thoughout the run of the game, while the [`Cid`] may change if the
    /// asset's content changes, for instance, due to hot reload.
    pub rid: Ulid,
}
impl<'a, Io: AssetIo> AssetLoadCtx<'a, Io> {
    /// Create a new load context for a new asset.
    pub fn new(
        server: &'a mut AssetServer,
        io: &'a Io,
        pack: Option<&'a str>,
        path: &'a Path,
    ) -> AssetLoadCtx<'a, Io> {
        AssetLoadCtx {
            server,
            io,
            pack,
            path,
            dependencies: Vec::new(),
            cid: Cid::default(),
            rid: Ulid::new(),
        }
    }
}

/// A generic value type for asset metadata.
#[derive(Debug, Clone, TypeUlid)]
#[ulid = "01H47PH3H6DM98E4KDWC2BVYYG"]
pub enum Metadata {
    /// A mapping.
    Map(HashMap<String, Metadata>),
    /// A list.
    List(Vec<Metadata>),
    /// An asset.
    Asset(UntypedHandle),
    /// A string.
    String(String),
    /// A number.
    Number(f64),
    /// A boolean.
    Bool(bool),
    /// A null value.
    Null,
}

/// Helper macro for generating `as_string` as `as_string_mut` kind of functions for [`Metadata`].
macro_rules! metadata_as_fn {
    ($doc_thing:literal, $variant:ty, $name:ident, $out:ty) => {
        paste::paste! {
            #[doc = concat!(
                "Get the ",
                $doc_thing,
                " if this is a [`",
                stringify!($variant),
                "`]."
            )]
            pub fn [< as_ $name >](&self) -> Option<&$out> {
                if let $variant(s) = self {
                    Some(s)
                } else {
                    None
                }
            }

            #[doc = concat!(
                "Get the ",
                $doc_thing,
                " if this is a [`",
                stringify!($variant),
                "`]."
            )]
            pub fn [< as_ $name _mut >](&mut self) -> Option<&mut $out> {
                if let $variant(s) = self {
                    Some(s)
                } else {
                    None
                }
            }
        }
    };
}

impl Metadata {
    /// Get the value with the given key, if this is a map and the key exists.
    pub fn get_key(&self, key: &str) -> Option<&Metadata> {
        self.as_map().and_then(|m| m.get(key))
    }

    /// Get the value at the given index, if this is a list and the index exists.
    pub fn get(&self, idx: usize) -> Option<&Metadata> {
        if let Metadata::List(l) = &self {
            l.get(idx)
        } else {
            None
        }
    }

    metadata_as_fn!("[`UntypedHandle`]", Metadata::Asset, asset, UntypedHandle);
    metadata_as_fn!("[`String`]", Metadata::String, string, String);
    metadata_as_fn!("[`bool`]", Metadata::Bool, bool, bool);
    metadata_as_fn!("[`f64`]", Metadata::Number, number, f64);
    metadata_as_fn!("[`HashMap<String, Metadata>`]", Metadata::Map, map, HashMap<String, Metadata>);
    metadata_as_fn!("[`Vec<Metadata>`]", Metadata::List, vec, Vec<Metadata>);
}

impl<'a, 'server, 'de, Io: AssetIo> serde::de::DeserializeSeed<'de>
    for &'a AssetLoadCtxCell<'server, Io>
{
    type Value = Metadata;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(self)
    }
}

impl<'a, 'server, 'de, Io: AssetIo> serde::de::Visitor<'de> for &'a AssetLoadCtxCell<'server, Io> {
    type Value = Metadata;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            formatter,
            "a map, a list, an asset, a string, a number, a bool, or null"
        )
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut data = HashMap::with_capacity(map.size_hint().unwrap_or(0));
        while let Some((key, value)) = map.next_entry_seed(PhantomData, self)? {
            data.insert(key, value);
        }
        let meta = Metadata::Map(data);

        Ok(meta)
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut data = Vec::with_capacity(seq.size_hint().unwrap_or(0));
        while let Some(value) = seq.next_element_seed(self)? {
            data.push(value);
        }
        Ok(Metadata::List(data))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        self.visit_string(v.to_owned())
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        if let Some(asset_path) = v.strip_prefix("asset:") {
            let mut ctx = self.0.try_borrow_mut().unwrap();

            let AssetLoadCtx {
                server,
                io,
                pack,
                dependencies,
                path,
                ..
            } = &mut *ctx;

            let normalized_path = normalize_relative_to(Path::new(asset_path), path);

            let handle = io
                .load_asset(server, &normalized_path, *pack)
                .map_err(|e| serde::de::Error::custom(format!("{e}")))?;
            let asset = server.get_untyped(&handle).unwrap();
            dependencies.push(asset.cid);

            Ok(Metadata::Asset(handle))
        } else {
            Ok(Metadata::String(v))
        }
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Metadata::Number(v as f64))
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Metadata::Number(v as f64))
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Metadata::Number(v))
    }
}

/// Normalize `path` relative to the `base_path`.
pub fn normalize_relative_to(path: &Path, base_path: &Path) -> PathBuf {
    fn normalize_path(path: &std::path::Path) -> PathBuf {
        let mut components = path.components().peekable();
        let mut ret = if let Some(c @ std::path::Component::Prefix(..)) = components.peek() {
            let buf = std::path::PathBuf::from(c.as_os_str());
            components.next();
            buf
        } else {
            std::path::PathBuf::new()
        };

        for component in components {
            match component {
                std::path::Component::Prefix(..) => unreachable!(),
                std::path::Component::RootDir => {
                    ret.push(component.as_os_str());
                }
                std::path::Component::CurDir => {}
                std::path::Component::ParentDir => {
                    ret.pop();
                }
                std::path::Component::Normal(c) => {
                    ret.push(c);
                }
            }
        }

        ret
    }

    let is_relative = !path.starts_with(Path::new("/"));

    let path = if is_relative {
        let base = base_path.parent().unwrap_or_else(|| Path::new(""));
        base.join(path)
    } else {
        path.to_path_buf()
    };

    normalize_path(&path)
}
