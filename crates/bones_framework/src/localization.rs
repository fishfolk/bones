//! Localization module.

use std::{borrow::Cow, path::PathBuf, str::FromStr, sync::Arc};

use bones_lib::prelude::anyhow::Context;
use fluent_bundle::FluentResource;
use intl_memoizer::concurrent::IntlLangMemoizer;
use unic_langid::LanguageIdentifier;

use crate::prelude::*;

pub use fluent_bundle;
pub use fluent_langneg;
pub use intl_memoizer;
pub use sys_locale;
pub use unic_langid;

/// Specialization of of the fluent bundle that is used by bones_framework.
pub type FluentBundle = fluent_bundle::bundle::FluentBundle<FluentResourceAsset, IntlLangMemoizer>;

/// An asset containing a [`FluentResource`].
#[derive(HasSchema, Deref, DerefMut, Clone)]
#[schema(opaque, no_default)]
#[type_data(asset_loader(["ftl"], FluentResourceLoader))]
pub struct FluentResourceAsset(pub Arc<FluentResource>);
impl std::borrow::Borrow<FluentResource> for FluentResourceAsset {
    fn borrow(&self) -> &FluentResource {
        &self.0
    }
}

/// An asset containing a [`FluentBundle`].
#[derive(HasSchema, Deref, DerefMut, Clone)]
#[schema(opaque, no_default)]
#[type_data(asset_loader(["locale.yaml", "locale.yml"], FluentBundleLoader))]
pub struct FluentBundleAsset(pub Arc<FluentBundle>);

/// Asset containing all loaded localizations, and functions for formatting localized messages.
#[derive(HasSchema, Deref, DerefMut, Clone)]
#[schema(opaque, no_default)]
#[type_data(asset_loader(["localization.yaml", "localization.yml"], LocalizationLoader))]
pub struct Localization {
    /// The bundle selected as the current language.
    #[deref]
    pub current_bundle: FluentBundleAsset,
    /// The bundles for all loaded languages.
    pub bundles: Arc<[FluentBundleAsset]>,
}

impl Localization {
    /// Get a localized message.
    pub fn get(&self, id: &str) -> Cow<'_, str> {
        let b = &self.current_bundle.0;
        let Some(message) = b.get_message(id) else {
            return Cow::from("")
        };

        b.format_pattern(message.value().unwrap(), None, &mut vec![])
    }
}

struct FluentResourceLoader;
impl AssetLoader for FluentResourceLoader {
    fn load(&self, _ctx: AssetLoadCtx, bytes: &[u8]) -> anyhow::Result<SchemaBox> {
        let string =
            String::from_utf8(bytes.to_vec()).context("Error loading fluent resource file.")?;
        let res = FluentResource::try_new(string).map_err(|(_, errors)| {
            let errors = errors
                .into_iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join("\n");

            anyhow::format_err!("Error loading fluent resource file. \n{}", errors)
        })?;

        Ok(SchemaBox::new(FluentResourceAsset(Arc::new(res))))
    }
}

struct FluentBundleLoader;
impl AssetLoader for FluentBundleLoader {
    fn load(&self, mut ctx: AssetLoadCtx, bytes: &[u8]) -> anyhow::Result<SchemaBox> {
        let self_path = ctx.path;
        #[derive(serde::Serialize, serde::Deserialize)]
        struct BundleMeta {
            pub locales: Vec<LanguageIdentifier>,
            pub resources: Vec<PathBuf>,
        }
        let meta: BundleMeta =
            serde_yaml::from_slice(bytes).context("Could not parse locale YAML")?;

        let mut bundle = FluentBundle::new_concurrent(meta.locales);

        for resource_path in meta.resources {
            let normalized = normalize_path_relative_to(&resource_path, self_path);
            let resource_handle = ctx.load_asset(&normalized)?.typed::<FluentResourceAsset>();
            let resource = ctx.asset_server.get(resource_handle);
            bundle.add_resource(resource.clone()).map_err(|e| {
                anyhow::format_err!(
                    "Error(s) adding resource `{normalized:?}` to bundle `{self_path:?}`: {e:?}"
                )
            })?;
        }

        Ok(SchemaBox::new(FluentBundleAsset(Arc::new(bundle))))
    }
}

struct LocalizationLoader;
impl AssetLoader for LocalizationLoader {
    fn load(&self, mut ctx: AssetLoadCtx, bytes: &[u8]) -> anyhow::Result<SchemaBox> {
        let self_path = ctx.path;
        #[derive(serde::Serialize, serde::Deserialize)]
        struct LocalizationMeta {
            pub locales: Vec<PathBuf>,
        }
        let meta: LocalizationMeta =
            serde_yaml::from_slice(bytes).context("Could not parse locale YAML")?;

        let mut bundles = Vec::new();

        for bundle_path in meta.locales {
            let normalized = normalize_path_relative_to(&bundle_path, self_path);
            let bundle_handle = ctx.load_asset(&normalized)?.typed::<FluentBundleAsset>();
            let bundle = ctx.asset_server.get(bundle_handle);
            bundles.push(bundle.clone());
        }

        let available_locales = bundles
            .iter()
            .flat_map(|x| x.locales.iter())
            .cloned()
            .collect::<Vec<_>>();

        let en_us = LanguageIdentifier::from_str("en-US").unwrap();
        let user_locale = sys_locale::get_locale()
            .and_then(|x| x.parse::<LanguageIdentifier>().ok())
            .unwrap_or(en_us.clone());

        let selected_locale = fluent_langneg::negotiate_languages(
            &[user_locale.clone()],
            &available_locales,
            Some(&en_us),
            fluent_langneg::NegotiationStrategy::Filtering,
        )[0];

        let selected_bundle = bundles
            .iter()
            .find(|bundle| bundle.locales.contains(selected_locale))
            .ok_or_else(|| {
                anyhow::format_err!("Could not find matching locale for {user_locale}")
            })?;

        Ok(SchemaBox::new(Localization {
            current_bundle: selected_bundle.clone(),
            bundles: bundles.into_iter().collect(),
        }))
    }
}
