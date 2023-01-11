#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::prelude::*;

use std::{collections::HashMap, sync::Arc};

pub(crate) mod prelude {
    pub use super::AnimatedSprite;
}

pub fn install(stages: &mut SystemStages) {
    stages
        .add_system_to_stage(CoreStage::Last, update_animation_banks)
        .add_system_to_stage(CoreStage::Last, animate_sprites);
}

/// Component that may be added to entities with an [`AtlasSprite`] to animate them.
#[derive(Clone, TypeUlid, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[ulid = "01GNZRPWKAHKP33V1KKRVAMVS7"]
pub struct AnimatedSprite {
    pub start: usize,
    pub end: usize,
    pub fps: f32,
    #[cfg_attr(feature = "serde", serde(default))]
    pub timer: f32,
    #[cfg_attr(feature = "serde", serde(default = "default_true"))]
    pub repeat: bool,
}

#[cfg(feature = "serde")]
fn default_true() -> bool {
    true
}

#[derive(Clone, TypeUlid, Debug)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[ulid = "01GP4EM4BGJPX22HYAMGYKKSAV"]
pub struct AnimationBankSprite {
    #[cfg_attr(feature = "serde", serde(default))]
    /// The current animation.
    pub current: Key,
    /// The collection of animations in this animation bank.
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_arc"))]
    #[cfg_attr(feature = "serde", serde(serialize_with = "serialize_arc"))]
    pub animations: Arc<HashMap<Key, AnimatedSprite>>,
    #[cfg_attr(feature = "serde", serde(default))]
    /// The last animation that was playing.
    pub last_animation: Key,
}

#[cfg(feature = "serde")]
fn deserialize_arc<'de, T: Deserialize<'de>, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Arc<T>, D::Error> {
    let item = T::deserialize(deserializer)?;
    Ok(Arc::new(item))
}
#[cfg(feature = "serde")]
fn serialize_arc<T: Serialize + Clone, S: Serializer>(
    data: &Arc<T>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    use std::ops::Deref;
    let data = data.deref().clone();
    data.serialize(serializer)
}

impl Default for AnimatedSprite {
    fn default() -> Self {
        Self {
            start: 0,
            end: 0,
            fps: 0.0,
            timer: 0.0,
            repeat: true,
        }
    }
}

/// System for automatically animating sprites with the [`AnimatedSprite`] component.
pub fn animate_sprites(
    entities: Res<Entities>,
    frame_time: Res<FrameTime>,
    mut atlas_sprites: CompMut<AtlasSprite>,
    mut animated_sprites: CompMut<AnimatedSprite>,
) {
    for (_ent, (atlas_sprite, animated_sprite)) in
        entities.iter_with((&mut atlas_sprites, &mut animated_sprites))
    {
        animated_sprite.timer += **frame_time;

        if animated_sprite.timer > 1.0 / animated_sprite.fps.max(f32::MIN_POSITIVE) {
            animated_sprite.timer = 0.0;
            if atlas_sprite.index >= animated_sprite.end {
                if animated_sprite.repeat {
                    atlas_sprite.index = animated_sprite.start;
                } else {
                    atlas_sprite.index = animated_sprite.end;
                }
            } else {
                atlas_sprite.index += 1;
            }
        }
    }
}

/// System for updating [`AnimatedSprite`]s based on thier [`AnimationBankSprite`]
pub fn update_animation_banks(
    entities: Res<Entities>,
    mut animation_bank_sprites: CompMut<AnimationBankSprite>,
    mut animated_sprites: CompMut<AnimatedSprite>,
    mut atlas_sprites: CompMut<AtlasSprite>,
) {
    for (ent, (animation_bank, atlas_sprite)) in
        entities.iter_with((&mut animation_bank_sprites, &mut atlas_sprites))
    {
        if animation_bank.current != animation_bank.last_animation {
            animation_bank.last_animation = animation_bank.current;
            let mut animated_sprite = *animation_bank
                .animations
                .get(&animation_bank.current)
                .unwrap_or_else(|| {
                    panic!("Animation `{}` does not exist.", animation_bank.current)
                });

            // Force the animation to restart
            animated_sprite.timer = f32::MAX;
            atlas_sprite.index = animated_sprite.start;

            animated_sprites.insert(ent, animated_sprite);
        }
    }
}
