//! Localization module.

use std::{
    borrow::Cow,
    marker::PhantomData,
    path::PathBuf,
    str::FromStr,
    sync::{Arc, OnceLock},
};

use crate::prelude::*;

use fluent::{FluentArgs, FluentResource};
use intl_memoizer::concurrent::IntlLangMemoizer;
use unic_langid::LanguageIdentifier;

pub use fluent;
pub use fluent::fluent_args;
pub use fluent_langneg;
pub use intl_memoizer;
pub use sys_locale;
pub use unic_langid;

/// Specialization of of the fluent bundle that is used by bones_framework.
pub type FluentBundle = fluent::bundle::FluentBundle<FluentResourceAsset, IntlLangMemoizer>;

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
pub struct LocalizationAsset {
    /// The bundle selected as the current language.
    #[deref]
    pub current_bundle: FluentBundleAsset,
    /// The bundles for all loaded languages.
    pub bundles: Arc<[FluentBundleAsset]>,
}

impl LocalizationAsset {
    /// Get a localized message.
    pub fn get(&self, id: &str) -> Cow<'_, str> {
        let b = &self.current_bundle.0;
        let Some(message) = b.get_message(id) else {
            return Cow::from("");
        };
        let Some(value) = message.value() else {
            return Cow::from("");
        };

        // TODO: Log localization formatting errors.
        // We need to find a way to log the errors without allocating every time we format:
        // https://github.com/projectfluent/fluent-rs/issues/323.
        b.format_pattern(value, None, &mut vec![])
    }

    /// Get a localized message with the provided arguments.
    pub fn get_with<'a>(&'a self, id: &'a str, args: &'a FluentArgs) -> Cow<'a, str> {
        let b = &self.current_bundle.0;
        let Some(message) = b.get_message(id) else {
            return Cow::from("");
        };
        let Some(value) = message.value() else {
            return Cow::from("");
        };

        b.format_pattern(value, Some(args), &mut vec![])
    }
}

use dashmap::mapref::one::MappedRef;
/// Borrow the localization field from the root asset.
///
/// This parameter uses the schema implementation to find the field of the root asset that is a
/// [`Handle<LocalizationAsset>`].
#[derive(Deref, DerefMut)]
pub struct Localization<'a, T> {
    #[deref]
    asset: MappedRef<'a, Cid, LoadedAsset, LocalizationAsset>,
    _phantom: PhantomData<T>,
}

/// Internal resource used to cache the field of the root asset containing the localization resource
/// for the [`Localization`] parameter.
#[derive(HasSchema, Default, Clone)]
pub struct RootLocalizationFieldIdx(OnceLock<usize>);

impl<T: HasSchema> SystemParam for Localization<'_, T> {
    type State = (AssetServer, AtomicResource<RootLocalizationFieldIdx>);
    type Param<'s> = Localization<'s, T>;

    fn get_state(world: &World) -> Self::State {
        (
            (*world.resources.get::<AssetServer>().unwrap()).clone(),
            world.resources.get_cell::<RootLocalizationFieldIdx>(),
        )
    }
    fn borrow<'s>(
        world: &'s World,
        (asset_server, field_idx): &'s mut Self::State,
    ) -> Self::Param<'s> {
        const ERR: &str = "Could not find a `Handle<LocalizationAsset>` field on root asset, \
                           needed for `Localization` parameter to work";
        let field_idx = field_idx.init_borrow(world);
        let field_idx = field_idx.0.get_or_init(|| {
            let mut idx = None;
            for (i, field) in T::schema()
                .kind
                .as_struct()
                .expect(ERR)
                .fields
                .iter()
                .enumerate()
            {
                if let Some(handle_data) = field.schema.type_data.get::<SchemaAssetHandle>() {
                    if let Some(schema) = handle_data.inner_schema() {
                        if schema == LocalizationAsset::schema() {
                            idx = Some(i);
                            break;
                        }
                    }
                }
            }
            idx.expect(ERR)
        });

        let root = asset_server.root::<T>();
        let root = root.as_schema_ref();
        let handle = root
            .field(*field_idx)
            .expect(ERR)
            .cast::<Handle<LocalizationAsset>>();
        let asset = asset_server.get(*handle);

        Localization {
            asset,
            _phantom: PhantomData,
        }
    }
}

struct FluentResourceLoader;
impl AssetLoader for FluentResourceLoader {
    fn load(&self, _ctx: AssetLoadCtx, bytes: &[u8]) -> BoxedFuture<anyhow::Result<SchemaBox>> {
        let bytes = bytes.to_vec();
        Box::pin(async move {
            let string = String::from_utf8(bytes).context("Error loading fluent resource file.")?;
            let res = FluentResource::try_new(string).map_err(|(_, errors)| {
                let errors = errors
                    .into_iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join("\n");

                anyhow::format_err!("Error loading fluent resource file. \n{}", errors)
            })?;

            Ok(SchemaBox::new(FluentResourceAsset(Arc::new(res))))
        })
    }
}

struct FluentBundleLoader;
impl AssetLoader for FluentBundleLoader {
    fn load(&self, mut ctx: AssetLoadCtx, bytes: &[u8]) -> BoxedFuture<anyhow::Result<SchemaBox>> {
        let bytes = bytes.to_vec();
        Box::pin(async move {
            let self_path = ctx.loc.path.clone();
            #[derive(serde::Serialize, serde::Deserialize)]
            struct BundleMeta {
                pub locales: Vec<LanguageIdentifier>,
                pub resources: Vec<PathBuf>,
            }
            let meta: BundleMeta =
                serde_yaml::from_slice(&bytes).context("Could not parse locale YAML")?;

            let mut bundle = FluentBundle::new_concurrent(meta.locales);

            for resource_path in meta.resources {
                let normalized = resource_path
                    .absolutize_from(self_path.parent().unwrap())
                    .unwrap();
                let resource_handle = ctx.load_asset(&normalized)?.typed::<FluentResourceAsset>();
                let resource = loop {
                    if let Some(resource) = ctx.asset_server.try_get(resource_handle) {
                        break resource.context("FluentBundle resource `{normalized:?}` handle could not be cast to `FluentResourceAsset`")?;
                    }
                    ctx.asset_server.load_progress.listen().await;
                };
                bundle.add_resource(resource.clone()).map_err(|e| {
                    anyhow::format_err!(
                    "Error(s) adding resource `{normalized:?}` to bundle `{self_path:?}`: {e:?}"
                )
                })?;
            }

            Ok(SchemaBox::new(FluentBundleAsset(Arc::new(bundle))))
        })
    }
}

struct LocalizationLoader;
impl AssetLoader for LocalizationLoader {
    fn load(&self, mut ctx: AssetLoadCtx, bytes: &[u8]) -> BoxedFuture<anyhow::Result<SchemaBox>> {
        let bytes = bytes.to_vec();
        Box::pin(async move {
            let self_path = ctx.loc.path.clone();
            #[derive(serde::Serialize, serde::Deserialize)]
            struct LocalizationMeta {
                pub locales: Vec<PathBuf>,
            }
            let meta: LocalizationMeta =
                serde_yaml::from_slice(&bytes).context("Could not parse locale YAML")?;

            let mut bundles: Vec<FluentBundleAsset> = Vec::new();

            for bundle_path in meta.locales {
                let normalized = bundle_path
                    .absolutize_from(self_path.parent().unwrap())
                    .unwrap();
                let bundle_handle = ctx.load_asset(&normalized)?.typed::<FluentBundleAsset>();
                let bundle = loop {
                    if let Some(bundle) = ctx.asset_server.try_get(bundle_handle) {
                        break bundle.context("Localization resource `{normalized:?}` loaded handle could not be cast to `FluentBundleAsset`")?;
                    }
                    ctx.asset_server.load_progress.listen().await;
                };
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

            Ok(SchemaBox::new(LocalizationAsset {
                current_bundle: selected_bundle.clone(),
                bundles: bundles.into_iter().collect(),
            }))
        })
    }
}
