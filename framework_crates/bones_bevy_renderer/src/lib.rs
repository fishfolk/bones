//! Bevy plugin for rendering Bones framework games.

#![warn(missing_docs)]
// This cfg_attr is needed because `rustdoc::all` includes lints not supported on stable
#![cfg_attr(doc, allow(unknown_lints))]
#![deny(rustdoc::all)]

use std::path::PathBuf;

pub use bevy;

use bevy::{
    input::{
        gamepad::GamepadEvent,
        keyboard::KeyboardInput,
        mouse::{MouseButtonInput, MouseMotion, MouseWheel},
    },
    prelude::*,
    render::{camera::ScalingMode, Extract, RenderApp},
    sprite::{extract_sprites, Anchor, ExtractedSprite, ExtractedSprites, SpriteSystem},
    utils::{HashMap, Instant},
};
use bevy_egui::EguiContext;
use glam::*;

use bevy_prototype_lyon::prelude as lyon;
use bones_framework::prelude::{self as bones, SchemaBox};
use prelude::convert::{IntoBevy, IntoBones};
use serde::{de::Visitor, Deserialize, Serialize};

/// The prelude
pub mod prelude {
    pub use crate::*;
}

mod convert;

/// Marker component for entities that are rendered in Bevy for bones.
#[derive(Component)]
pub struct BevyBonesEntity;

/// Renderer for [`bones_framework`] [`Game`][bones::Game]s using Bevy.
#[derive(Resource)]
pub struct BonesBevyRenderer {
    /// Whether or not to use nearest-neighbor sampling for textures.
    pub pixel_art: bool,
    /// The bones game to run.
    pub game: bones::Game,
    /// The version of the game, used for the asset loader.
    pub game_version: bones::Version,
    /// The (qualifier, organization, application) that will be used to pick a persistent storage
    /// location for the game.
    ///
    /// For example: `("org", "fishfolk", "jumpy")`
    pub app_namespace: (String, String, String),
    /// The path to load assets from.
    pub asset_dir: PathBuf,
    /// The path to load asset packs from.
    pub packs_dir: PathBuf,
}

/// Resource containing the entity spawned for all of the bones game renderables.
#[derive(Resource)]
pub struct BonesGameEntity(pub Entity);
impl FromWorld for BonesGameEntity {
    fn from_world(world: &mut World) -> Self {
        Self(world.spawn(VisibilityBundle::default()).id())
    }
}

/// Resource mapping bones image IDs to their bevy handles.
#[derive(Resource, Debug, Deref, DerefMut)]
pub struct BonesImageIds {
    #[deref]
    map: HashMap<u32, Handle<Image>>,
    next_id: u32,
}
impl Default for BonesImageIds {
    fn default() -> Self {
        Self {
            map: Default::default(),
            next_id: 1,
        }
    }
}

impl BonesImageIds {
    /// Load all bones images into bevy.
    pub fn load_bones_images(
        &mut self,
        bones_assets: &mut bones::AssetServer,
        bones_egui_textures: &mut bones::EguiTextures,
        bevy_images: &mut Assets<Image>,
        bevy_egui_textures: &mut bevy_egui::EguiUserTextures,
    ) {
        for (handle, cid) in bones_assets.store.asset_ids.iter() {
            let asset = bones_assets.store.assets.get_mut(cid).unwrap();
            if let Ok(image) = asset.data.try_cast_mut::<bones::Image>() {
                self.load_bones_image(
                    handle.typed(),
                    image,
                    bones_egui_textures,
                    bevy_images,
                    bevy_egui_textures,
                )
            }
        }
    }

    /// Load a bones image into bevy.
    pub fn load_bones_image(
        &mut self,
        bones_handle: bones::Handle<bones::Image>,
        image: &mut bones::Image,
        bones_egui_textures: &mut bones::EguiTextures,
        bevy_images: &mut Assets<Image>,
        bevy_egui_textures: &mut bevy_egui::EguiUserTextures,
    ) {
        let Self { map, next_id } = self;
        let mut taken_image = bones::Image::External(0); // Dummy value temporarily
        std::mem::swap(image, &mut taken_image);
        if let bones::Image::Data(data) = taken_image {
            let handle = bevy_images.add(Image::from_dynamic(data, true));
            let egui_texture = bevy_egui_textures.add_image(handle.clone());
            bones_egui_textures.insert(bones_handle, egui_texture);
            map.insert(*next_id, handle);
            *image = bones::Image::External(*next_id);
            *next_id += 1;

        // The image has already been loaded. This may happen if multiple asset handles use the same
        // image data. We will end up visiting the same data twice.
        } else {
            // Swap the image back to it's previous value.
            std::mem::swap(image, &mut taken_image);
        }
    }
}

fn update_egui_fonts(ctx: &bevy_egui::egui::Context, bones_assets: &bones::AssetServer) {
    use bevy_egui::egui;
    let mut fonts = egui::FontDefinitions::default();

    for asset in bones_assets.store.assets.values() {
        if let Ok(font) = asset.try_cast_ref::<bones::Font>() {
            let previous = fonts
                .font_data
                .insert(font.family_name.to_string(), font.data.clone());
            if previous.is_some() {
                warn!(
                    name=%font.family_name,
                    "Found two fonts with the same family name, using \
                    only the latest one"
                );
            }
            fonts
                .families
                .entry(egui::FontFamily::Name(font.family_name.clone()))
                .or_default()
                .push(font.family_name.to_string());
        }
    }

    ctx.set_fonts(fonts);
}

/// Bevy resource that contains the info for the bones game that is being rendered.
#[derive(Resource)]
pub struct BonesData {
    /// The bones game.
    pub game: bones::Game,
    /// The bones asset server cell.
    pub asset_server: Option<bones::AtomicResource<bones::AssetServer>>,
    /// The bones egui texture resource.
    pub bones_egui_textures: bones::AtomicResource<bones::EguiTextures>,
}

impl BonesBevyRenderer {
    // TODO: Create a better builder pattern struct for `BonesBevyRenderer`.
    // We want to use a nice builder-pattern struct for `BonesBevyRenderer` so that it is easier
    // to set options like the `pixel_art` flag or the `game_version`.
    /// Create a new [`BonesBevyRenderer`] for the provided game.
    pub fn new(game: bones::Game) -> Self {
        BonesBevyRenderer {
            pixel_art: true,
            game,
            game_version: bones::Version::new(0, 1, 0),
            app_namespace: ("org".into(), "fishfolk".into(), "bones_demo_game".into()),
            asset_dir: PathBuf::from("assets"),
            packs_dir: PathBuf::from("packs"),
        }
    }

    /// Return a bevy [`App`] configured to run the bones game.
    pub fn app(mut self) -> App {
        let mut app = App::new();

        // Initialize Bevy plugins we use
        let mut plugins = DefaultPlugins.build();
        if self.pixel_art {
            plugins = plugins.set(ImagePlugin::default_nearest());
        }

        app.add_plugins(plugins)
            .add_plugins((bevy_egui::EguiPlugin, lyon::ShapePlugin))
            .insert_resource({
                let mut egui_settings = bevy_egui::EguiSettings::default();
                if self.pixel_art {
                    egui_settings.use_nearest_descriptor();
                }
                egui_settings
            })
            .init_resource::<BonesImageIds>();

        let mut bones_image_ids = BonesImageIds::default();
        let mut bones_egui_textures = bones::EguiTextures::default();
        'asset_load: {
            let Some(mut asset_server) = self.game.shared_resource_mut::<bones::AssetServer>()
            else {
                break 'asset_load;
            };

            let world = app.world.cell();
            let mut bevy_images = world.resource_mut::<Assets<Image>>();
            let mut bevy_egui_textures = world.resource_mut::<bevy_egui::EguiUserTextures>();

            #[cfg(not(target_arch = "wasm32"))]
            {
                // Configure the AssetIO
                let io = bones::FileAssetIo::new(&self.asset_dir, &self.packs_dir, true);
                asset_server.set_io(io);
            }

            // Load the game assets
            asset_server
                .load_assets()
                .expect("Could not load game assets");

            // Take all loaded image assets and conver them to external images that reference bevy handles
            bones_image_ids.load_bones_images(
                &mut asset_server,
                &mut bones_egui_textures,
                &mut bevy_images,
                &mut bevy_egui_textures,
            );
        }

        // Configure and load the persitent storage
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut storage = bones::Storage::with_backend(Box::new(StorageBackend::new(
                &self.app_namespace.0,
                &self.app_namespace.1,
                &self.app_namespace.2,
            )));
            storage.load();
            self.game.insert_shared_resource(storage);
        }

        self.game.insert_shared_resource(bones_egui_textures);
        app.insert_resource(bones_image_ids);

        // Insert the bones data
        app.insert_resource(BonesData {
            asset_server: self.game.shared_resource_cell::<bones::AssetServer>(),
            bones_egui_textures: self
                .game
                .shared_resource_cell::<bones::EguiTextures>()
                .unwrap(),
            game: self.game,
        })
        .init_resource::<BonesGameEntity>();

        // Add the world sync systems
        app.add_systems(Startup, setup_egui).add_systems(
            Update,
            (
                // Collect input and run world simulation
                get_bones_input.pipe(step_bones_game),
                // Synchronize bones render components with the Bevy world.
                (
                    sync_egui_settings,
                    sync_clear_color,
                    sync_cameras,
                    sync_bones_path2ds,
                ),
            )
                .chain(),
        );

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.add_systems(
                ExtractSchedule,
                (extract_bones_sprites, extract_bones_tilemaps)
                    .in_set(SpriteSystem::ExtractSprites)
                    .after(extract_sprites),
            );
        }

        app
    }
}

#[cfg(not(target_arch = "wasm32"))]
struct StorageBackend {
    storage_path: PathBuf,
}
#[cfg(not(target_arch = "wasm32"))]
impl StorageBackend {
    fn new(qualifier: &str, organization: &str, application: &str) -> Self {
        let project_dirs = directories::ProjectDirs::from(qualifier, organization, application)
            .expect("Identify system data dir path");
        Self {
            storage_path: project_dirs.data_dir().join("storage.yml"),
        }
    }
}
#[cfg(not(target_arch = "wasm32"))]
impl bones::StorageApi for StorageBackend {
    fn save(&mut self, data: Vec<bones::SchemaBox>) {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(&self.storage_path)
            .expect("Failed to open storage file");
        let mut serializer = serde_yaml::Serializer::new(file);
        LoadedStorage(data)
            .serialize(&mut serializer)
            .expect("Failed to serialize to storage file.");
    }

    fn load(&mut self) -> Vec<bones::SchemaBox> {
        use anyhow::Context;
        if self.storage_path.exists() {
            let result: anyhow::Result<LoadedStorage> = (|| {
                let file = std::fs::OpenOptions::new()
                    .read(true)
                    .open(&self.storage_path)
                    .context("Failed to open storage file")?;
                let loaded: LoadedStorage =
                    serde_yaml::from_reader(file).context("Failed to deserialize storage file")?;

                anyhow::Result::Ok(loaded)
            })();
            match result {
                Ok(loaded) => loaded.0,
                Err(e) => {
                    error!(
                        "Error deserializing storage file, ignoring file, \
                        data will be overwritten when saved: {e:?}"
                    );
                    default()
                }
            }
        } else {
            std::fs::create_dir_all(self.storage_path.parent().unwrap()).unwrap();
            default()
        }
    }
}
struct LoadedStorage(Vec<SchemaBox>);
impl Serialize for LoadedStorage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let data: HashMap<String, bones::SchemaRef> = self
            .0
            .iter()
            .map(|x| (x.schema().full_name.to_string(), x.as_ref()))
            .collect();

        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(data.len()))?;

        for (key, value) in data {
            map.serialize_key(&key)?;
            map.serialize_value(&bones::SchemaSerializer(value))?;
        }

        map.end()
    }
}
impl<'de> Deserialize<'de> for LoadedStorage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(LoadedStorageVisitor).map(Self)
    }
}
struct LoadedStorageVisitor;
impl<'de> Visitor<'de> for LoadedStorageVisitor {
    type Value = Vec<SchemaBox>;
    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "Mapping of string type names to type data.")
    }
    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut data = Vec::new();
        while let Some(type_name) = map.next_key::<String>()? {
            let reg = bones::SCHEMA_REGISTRY.borrow();
            let Some(schema) = reg
                .schemas
                .iter()
                .map(|(_id, schema)| schema)
                .find(|schema| schema.full_name.as_ref() == type_name)
            else {
                error!(
                    "\n\nCannot find schema registration for `{}` while loading persisted \
                    storage. This means you that you need to call \
                    `{}::schema()` to register your persisted storage type before \
                    creating the `BonesBevyRenderer` or that there is data from an old \
                    version of the app inside of the persistent storage file.\n\n",
                    type_name, type_name,
                );
                continue;
            };

            data.push(map.next_value_seed(bones::SchemaDeserializer(schema))?);
        }

        Ok(data)
    }
}

/// Startup system to load egui fonts and textures.
fn setup_egui(world: &mut World) {
    world.resource_scope(|world: &mut World, mut bones_data: Mut<BonesData>| {
        let ctx = {
            let mut egui_query = world.query_filtered::<&mut EguiContext, With<Window>>();
            let mut egui_ctx = egui_query.get_single_mut(world).unwrap();
            egui_ctx.get_mut().clone()
        };

        // Insert the egui context as a shared resource
        bones_data
            .game
            .insert_shared_resource(bones::EguiCtx(ctx.clone()));

        if let Some(bones_assets) = &bones_data.asset_server {
            update_egui_fonts(&ctx, &bones_assets.borrow());

            // Insert the bones egui textures
            ctx.data_mut(|map| {
                map.insert_temp(
                    bevy_egui::egui::Id::null(),
                    bones_data.bones_egui_textures.clone(),
                );
            });
        }
    });
}

fn get_bones_input(
    mut mouse_button_input_events: EventReader<MouseButtonInput>,
    mut mouse_motion_events: EventReader<MouseMotion>,
    mut mouse_wheel_events: EventReader<MouseWheel>,
    mut keyboard_events: EventReader<KeyboardInput>,
    mut gamepad_events: EventReader<GamepadEvent>,
) -> (
    bones::MouseInputs,
    bones::KeyboardInputs,
    bones::GamepadInputs,
) {
    // TODO: investigate possible ways to avoid allocating vectors every frame for event lists.
    (
        bones::MouseInputs {
            movement: mouse_motion_events
                .iter()
                .last()
                .map(|x| x.delta)
                .unwrap_or_default(),
            wheel_events: mouse_wheel_events
                .iter()
                .map(|event| bones::MouseScrollEvent {
                    unit: event.unit.into_bones(),
                    movement: Vec2::new(event.x, event.y),
                })
                .collect(),
            button_events: mouse_button_input_events
                .iter()
                .map(|event| bones::MouseButtonEvent {
                    button: event.button.into_bones(),
                    state: event.state.into_bones(),
                })
                .collect(),
        },
        bones::KeyboardInputs {
            key_events: keyboard_events
                .iter()
                .map(|event| bones::KeyboardEvent {
                    scan_code: event.scan_code,
                    key_code: event.key_code.map(|x| x.into_bones()).into(),
                    button_state: event.state.into_bones(),
                })
                .collect(),
        },
        bones::GamepadInputs {
            gamepad_events: gamepad_events
                .iter()
                .map(|event| match event {
                    GamepadEvent::Connection(c) => {
                        bones::GamepadEvent::Connection(bones::GamepadConnectionEvent {
                            gamepad: c.gamepad.id as u32,
                            event: if c.connected() {
                                bones::GamepadConnectionEventKind::Connected
                            } else {
                                bones::GamepadConnectionEventKind::Disconnected
                            },
                        })
                    }
                    GamepadEvent::Button(b) => {
                        bones::GamepadEvent::Button(bones::GamepadButtonEvent {
                            gamepad: b.gamepad.id as u32,
                            button: b.button_type.into_bones(),
                            value: b.value,
                        })
                    }
                    GamepadEvent::Axis(a) => bones::GamepadEvent::Axis(bones::GamepadAxisEvent {
                        gamepad: a.gamepad.id as u32,
                        axis: a.axis_type.into_bones(),
                        value: a.value,
                    }),
                })
                .collect(),
        },
    )
}

/// System to step the bones simulation.
fn step_bones_game(
    In((mouse_inputs, keyboard_inputs, gamepad_inputs)): In<(
        bones::MouseInputs,
        bones::KeyboardInputs,
        bones::GamepadInputs,
    )>,
    world: &mut World,
) {
    let mut data = world.remove_resource::<BonesData>().unwrap();
    let mut bones_image_ids = world.remove_resource::<BonesImageIds>().unwrap();
    let mut bevy_egui_textures = world
        .remove_resource::<bevy_egui::EguiUserTextures>()
        .unwrap();
    let mut bevy_images = world.remove_resource::<Assets<Image>>().unwrap();

    let mut winow_query = world.query::<&Window>();
    let window = winow_query.get_single_mut(world).unwrap();
    let BonesData { game, .. } = &mut data;

    // Insert window information
    game.insert_shared_resource(bones::Window {
        size: vec2(window.width(), window.height()),
    });

    let bevy_time = world.resource::<Time>();

    // Reload assets if necessary
    if let Some(mut asset_server) = game.shared_resource_mut::<bones::AssetServer>() {
        asset_server.handle_asset_changes(|asset_server, handle| {
            let mut bones_egui_textures =
                game.shared_resource_mut::<bones::EguiTextures>().unwrap();
            let asset = asset_server.get_untyped_mut(handle).unwrap();

            // TODO: hot reload changed fonts.

            if let Ok(image) = asset.data.try_cast_mut::<bones::Image>() {
                bones_image_ids.load_bones_image(
                    handle.typed(),
                    image,
                    &mut bones_egui_textures,
                    &mut bevy_images,
                    &mut bevy_egui_textures,
                );
            }
        })
    }

    // Add the game inputs
    game.insert_shared_resource(mouse_inputs);
    game.insert_shared_resource(keyboard_inputs);
    game.insert_shared_resource(gamepad_inputs);

    // Step the game simulation
    game.step(bevy_time.last_update().unwrap_or_else(Instant::now));

    world.insert_resource(data);
    world.insert_resource(bones_image_ids);
    world.insert_resource(bevy_egui_textures);
    world.insert_resource(bevy_images);
}

fn sync_clear_color(mut clear_color: ResMut<ClearColor>, mut data: ResMut<BonesData>) {
    let game = &mut data.game;

    for name in &game.sorted_session_keys {
        let session = game.sessions.get(*name).unwrap();
        if !session.visible {
            continue;
        }
        if let Some(bones_clear_color) = session.world.get_resource::<bones::ClearColor>() {
            clear_color.0 = bones_clear_color.0.into_bevy();
        }
    }
}

fn sync_egui_settings(
    data: Res<BonesData>,
    mut bevy_egui_settings: ResMut<bevy_egui::EguiSettings>,
) {
    let game = &data.game;

    for session_name in &game.sorted_session_keys {
        let session = game.sessions.get(*session_name).unwrap();
        let world = &session.world;

        if let Some(settings) = world.get_resource::<bones::EguiSettings>() {
            bevy_egui_settings.scale_factor = settings.scale;
        }
    }
}

#[derive(Resource, Deref, DerefMut)]
struct CameraBuffer(Vec<(bones::Camera, bones::Transform)>);

/// Sync bones cameras with Bevy
fn sync_cameras(
    data: Res<BonesData>,
    mut commands: Commands,
    mut bevy_bones_cameras: Query<Entity, (With<BevyBonesEntity>, With<Camera>)>,
) {
    let game = &data.game;

    // Collect the bevy cameras that we've created for the bones game
    let mut bevy_bones_cameras = bevy_bones_cameras.iter_mut();

    // Create a helper callback to add/update a bones camera into the bevy world
    let mut add_bones_camera = |bones_camera: &bones::Camera,
                                bones_transform: &bones::Transform| {
        // Get or spawn an entity for the camera
        let mut camera_ent = match bevy_bones_cameras.next() {
            Some(ent) => commands.entity(ent),
            None => commands.spawn((Camera2dBundle::default(), BevyBonesEntity)),
        };

        // Insert our updated components on the camera
        camera_ent.insert((
            Camera {
                is_active: bones_camera.active,
                viewport: bones_camera.viewport.option().map(|x| x.into_bevy()),
                order: bones_camera.priority as isize,
                ..default()
            },
            OrthographicProjection {
                scaling_mode: match bones_camera.size {
                    bones::CameraSize::FixedHeight(h) => ScalingMode::FixedVertical(h),
                    bones::CameraSize::FixedWidth(w) => ScalingMode::FixedHorizontal(w),
                },
                ..default()
            },
            bones_transform.into_bevy(),
        ));
    };

    // Loop through all sessions
    for session_name in &game.sorted_session_keys {
        let session = game.sessions.get(*session_name).unwrap();
        if !session.visible {
            continue;
        }

        let world = &session.world;

        // Skip worlds without cameras and transforms
        if !(world.components.get_cell::<bones::Transform>().is_ok()
            && world.components.get_cell::<bones::Camera>().is_ok())
        {
            continue;
        }

        let entities = world.resource::<bones::Entities>();
        let transforms = world.components.get::<bones::Transform>().unwrap();
        let cameras = world.components.get::<bones::Camera>().unwrap();

        // Sync cameras
        for (_ent, (transform, camera)) in entities.iter_with((&transforms, &cameras)) {
            // Add each camera to the bevy world
            add_bones_camera(camera, transform)
        }
    }

    // Delete any remaining bevy cameras that don't have bones equivalents.
    for remaining_ent in bevy_bones_cameras {
        commands.entity(remaining_ent).despawn()
    }
}

fn extract_bones_sprites(
    mut extracted_sprites: ResMut<ExtractedSprites>,
    data: Extract<Res<BonesData>>,
    bones_image_ids: Extract<Res<BonesImageIds>>,
    bones_renderable_entity: Extract<Res<BonesGameEntity>>,
) {
    let game = &data.game;
    let Some(bones_assets) = &data.asset_server else {
        return;
    };
    let bones_assets = bones_assets.borrow();

    for session_name in &game.sorted_session_keys {
        let session = game.sessions.get(*session_name).unwrap();
        if !session.visible {
            continue;
        }

        let world = &session.world;

        // Skip worlds without cameras and transforms
        if !(world.components.get_cell::<bones::Transform>().is_ok()
            && world.components.get_cell::<bones::Camera>().is_ok()
            && (world.components.get_cell::<bones::Sprite>().is_ok()
                || world.components.get_cell::<bones::AtlasSprite>().is_ok()))
        {
            continue;
        }

        let entities = world.resource::<bones::Entities>();
        let transforms = world.components.get::<bones::Transform>().unwrap();

        // Extract normal sprites
        if let Ok(sprites) = world.components.get::<bones::Sprite>() {
            let mut z_offset = 0.0;
            for (_, (sprite, transform)) in entities.iter_with((&sprites, &transforms)) {
                let sprite_image = bones_assets.get(sprite.image);
                let image_id = if let bones::Image::External(id) = sprite_image {
                    *id
                } else {
                    panic!(
                        "Images added at runtime not supported yet, \
                please open an issue."
                    );
                };
                extracted_sprites.sprites.push(ExtractedSprite {
                    entity: bones_renderable_entity.0,
                    transform: {
                        let mut t: Transform = transform.into_bevy();
                        // Add tiny z offset to enforce a consistent z-sort
                        t.translation.z += z_offset;
                        z_offset += 0.00001;
                        t.into()
                    },
                    color: sprite.color.into_bevy(),
                    rect: None,
                    custom_size: None,
                    image_handle_id: bones_image_ids.get(&image_id).unwrap().id(),
                    flip_x: sprite.flip_x,
                    flip_y: sprite.flip_y,
                    anchor: Anchor::Center.as_vec(),
                })
            }
        }

        // Extract atlas sprites
        if let Ok(atlas_sprites) = world.components.get::<bones::AtlasSprite>() {
            for (_, (atlas_sprite, transform)) in entities.iter_with((&atlas_sprites, &transforms))
            {
                let atlas = bones_assets.get(atlas_sprite.atlas);
                let atlas_image = bones_assets.get(atlas.image);
                let image_id = if let bones::Image::External(id) = atlas_image {
                    *id
                } else {
                    panic!(
                        "Images added at runtime not supported yet, \
                        please open an issue."
                    );
                };
                let index = atlas_sprite.index;
                let y = index / atlas.columns;
                let x = index - (y * atlas.columns);
                let cell = Vec2::new(x as f32, y as f32);
                let current_padding = atlas.padding
                    * Vec2::new(if x > 0 { 1.0 } else { 0.0 }, if y > 0 { 1.0 } else { 0.0 });
                let min = (atlas.tile_size + current_padding) * cell + atlas.offset;
                let rect = Rect {
                    min,
                    max: min + atlas.tile_size,
                };
                extracted_sprites.sprites.push(ExtractedSprite {
                    entity: bones_renderable_entity.0,
                    transform: transform.into_bevy().into(),
                    color: atlas_sprite.color.into_bevy(),
                    rect: Some(rect),
                    custom_size: None,
                    image_handle_id: bones_image_ids.get(&image_id).unwrap().id(),
                    flip_x: atlas_sprite.flip_x,
                    flip_y: atlas_sprite.flip_y,
                    anchor: Anchor::Center.as_vec(),
                })
            }
        }
    }
}

fn extract_bones_tilemaps(
    mut extracted_sprites: ResMut<ExtractedSprites>,
    data: Extract<Res<BonesData>>,
    bones_image_ids: Extract<Res<BonesImageIds>>,
    bones_renderable_entity: Extract<Res<BonesGameEntity>>,
) {
    let game = &data.game;
    let Some(bones_assets) = &data.asset_server else {
        return;
    };
    let bones_assets = bones_assets.borrow();

    for session_name in &game.sorted_session_keys {
        let session = game.sessions.get(*session_name).unwrap();
        if !session.visible {
            continue;
        }

        let world = &session.world;

        // Skip worlds without cameras renderable tile layers
        if !(world.components.get_cell::<bones::Transform>().is_ok()
            && world.components.get_cell::<bones::Camera>().is_ok()
            && world.components.get_cell::<bones::TileLayer>().is_ok())
        {
            continue;
        }

        let entities = world.resource::<bones::Entities>();
        let transforms = world.components.get::<bones::Transform>().unwrap();
        let tile_layers = world.components.get::<bones::TileLayer>().unwrap();
        let tiles = world.components.get::<bones::Tile>().unwrap();

        // Extract tiles as sprites
        for (_, (tile_layer, transform)) in entities.iter_with((&tile_layers, &transforms)) {
            let atlas = bones_assets.get(tile_layer.atlas);
            let atlas_image = bones_assets.get(atlas.image);
            let image_id = if let bones::Image::External(id) = atlas_image {
                *id
            } else {
                panic!(
                    "Images added at runtime not supported yet, \
                        please open an issue."
                );
            };

            for (tile_pos_idx, tile_ent) in tile_layer.tiles.iter().enumerate() {
                let Some(tile_ent) = tile_ent else { continue };
                let tile = tiles.get(*tile_ent).unwrap();

                let tile_pos = tile_layer.pos(tile_pos_idx as u32);
                let tile_offset = tile_pos.as_vec2() * tile_layer.tile_size;

                let sprite_idx = tile.idx;
                let y = sprite_idx / atlas.columns;
                let x = sprite_idx - (y * atlas.columns);
                let cell = Vec2::new(x as f32, y as f32);
                let current_padding = atlas.padding
                    * Vec2::new(if x > 0 { 1.0 } else { 0.0 }, if y > 0 { 1.0 } else { 0.0 });
                let min = (atlas.tile_size + current_padding) * cell + atlas.offset;
                let rect = Rect {
                    min,
                    max: min + atlas.tile_size,
                };
                let mut transform = transform.into_bevy();
                transform.translation += tile_offset.extend(0.0);
                // Scale up slightly to avoid bleeding between tiles.
                // TODO: Improve tile rendering
                // Currently we do a small hack here, scaling up the tiles a little bit, to prevent
                // visible gaps between tiles. This solution isn't perfect and we probably need to
                // create a proper tile renderer. That can render multiple tiles on one quad instead
                // of using a separate quad for each tile.
                transform.scale += Vec3::new(0.01, 0.01, 0.0);
                extracted_sprites.sprites.push(ExtractedSprite {
                    entity: bones_renderable_entity.0,
                    transform: transform.into(),
                    color: Color::WHITE,
                    rect: Some(rect),
                    custom_size: None,
                    image_handle_id: bones_image_ids.get(&image_id).unwrap().id(),
                    flip_x: tile.flip_x,
                    flip_y: tile.flip_y,
                    anchor: Anchor::BottomLeft.as_vec(),
                })
            }
        }
    }
}

fn sync_bones_path2ds(
    data: Res<BonesData>,
    mut commands: Commands,
    mut bevy_bones_path2ds: Query<
        (Entity, &mut lyon::Path, &mut lyon::Stroke, &mut Transform),
        With<BevyBonesEntity>,
    >,
) {
    let game = &data.game;

    // Collect the bevy path2ds that we've created for the bones game
    let mut bevy_bones_path2ds = bevy_bones_path2ds.iter_mut();

    // Create a helper callback to add/update a bones path2d into the bevy world
    let mut add_bones_path2d = |bones_path2d: &bones::Path2d,
                                bones_transform: &bones::Transform| {
        // Get or create components for the entity
        let mut new_components = None;
        let mut existing_components;
        let (path, stroke, transform) = match bevy_bones_path2ds.next() {
            Some((_ent, path, stroke, transform)) => {
                existing_components = (path, stroke, transform);
                let (path, stroke, transform) = &mut existing_components;
                (&mut **path, &mut **stroke, &mut **transform)
            }
            None => {
                let bundle = lyon::ShapeBundle::default();
                new_components = Some((
                    bundle.path,
                    lyon::Stroke::new(Color::default(), 1.0),
                    bundle.transform,
                ));
                let (path, stroke, transform) = new_components.as_mut().unwrap();
                (path, stroke, transform)
            }
        };

        // Update the components
        *stroke = lyon::Stroke::new(bones_path2d.color.into_bevy(), bones_path2d.thickness);
        *path = bones_path2d
            .points
            .iter()
            .copied()
            .enumerate()
            .fold(lyon::PathBuilder::new(), |mut builder, (i, point)| {
                if i > 0 && !bones_path2d.line_breaks.contains(&i) {
                    builder.line_to(point);
                }
                builder.move_to(point);

                builder
            })
            .build();
        *transform = bones_transform.into_bevy();
        // Offset the path towards the camera slightly to make sure it renders on top of a
        // sprite/etc. if it is applied to an entity with both a sprite and a path.
        transform.translation.z += 0.0001;

        // Spawn the shape if it doesn't already exist
        if let Some((path, stroke, transform)) = new_components {
            commands
                .spawn(lyon::ShapeBundle {
                    path,
                    transform,
                    ..default()
                })
                .insert(stroke)
                .insert(BevyBonesEntity);
        }
    };

    for session_name in &game.sorted_session_keys {
        let session = game.sessions.get(*session_name).unwrap();
        if !session.visible {
            continue;
        }

        let world = &session.world;

        // Skip worlds without cameras renderable tile layers
        if !(world.components.get_cell::<bones::Transform>().is_ok()
            && world.components.get_cell::<bones::Camera>().is_ok()
            && world.components.get_cell::<bones::Path2d>().is_ok())
        {
            continue;
        }

        let entities = world.resource::<bones::Entities>();
        let transforms = world.components.get::<bones::Transform>().unwrap();
        let path2ds = world.components.get::<bones::Path2d>().unwrap();

        // Extract tiles as sprites
        for (_, (path2d, transform)) in entities.iter_with((&path2ds, &transforms)) {
            add_bones_path2d(path2d, transform);
        }
    }

    // Despawn extra path 2ds
    for (ent, ..) in bevy_bones_path2ds {
        commands.entity(ent).despawn()
    }
}
