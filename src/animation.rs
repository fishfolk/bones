//! Animation utilities and systems.

#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::prelude::*;

use std::sync::Arc;

pub(crate) mod prelude {
    pub use super::{AnimatedSprite, AnimationBankSprite};
}

/// Install animation utilities into the given [`SystemStages`].
pub fn plugin(core: &mut BonesCore) {
    core.stages
        .add_system_to_stage(CoreStage::Last, update_animation_banks)
        .add_system_to_stage(CoreStage::Last, animate_sprites);
}

/// Component that may be added to entities with an [`AtlasSprite`] to animate them.
#[derive(Clone, HasSchema, Debug)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[schema(opaque)]
pub struct AnimatedSprite {
    /// The current frame in the animation.
    #[cfg_attr(feature = "serde", serde(default))]
    pub index: usize,
    /// The frames in the animation.
    ///
    /// These are the indexes into the atlas, specified in the order they will be played.
    #[cfg_attr(feature = "serde", serde(deserialize_with = "deserialize_arc_slice"))]
    #[cfg_attr(feature = "serde", serde(serialize_with = "serialize_arc_slice"))]
    pub frames: Arc<[usize]>,
    /// The frames per second to play the animation at.
    pub fps: f32,
    /// The amount of time the current frame has been playing
    #[cfg_attr(feature = "serde", serde(default))]
    pub timer: f32,
    /// Whether or not to repeat the animation
    #[cfg_attr(feature = "serde", serde(default = "default_true"))]
    pub repeat: bool,
}

#[cfg(feature = "serde")]
fn default_true() -> bool {
    true
}

/// Component that may be added to an [`AtlasSprite`] to control which animation, out of a set of
/// animations, is playing.
///
/// This is great for players or other sprites that will change through different, named animations
/// at different times.
#[derive(Clone, HasSchema, Debug, Default)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[schema(opaque)]
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
    let data = data.deref();
    data.serialize(serializer)
}
#[cfg(feature = "serde")]
fn deserialize_arc_slice<'de, T: Deserialize<'de>, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Arc<[T]>, D::Error> {
    let item = <Vec<T>>::deserialize(deserializer)?;
    Ok(Arc::from(item))
}
#[cfg(feature = "serde")]
fn serialize_arc_slice<T: Serialize + Clone, S: Serializer>(
    data: &Arc<[T]>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    use std::ops::Deref;
    let data = data.deref();
    data.serialize(serializer)
}

impl Default for AnimatedSprite {
    fn default() -> Self {
        Self {
            index: 0,
            frames: Arc::from([]),
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
        if animated_sprite.frames.is_empty() {
            continue;
        }

        animated_sprite.timer += **frame_time;

        // If we are ready to go to the next frame
        if (animated_sprite.index != animated_sprite.frames.len() - 1 || animated_sprite.repeat)
            && animated_sprite.timer > 1.0 / animated_sprite.fps.max(f32::MIN_POSITIVE)
        {
            // Restart the timer
            animated_sprite.timer = 0.0;

            // Increment and loop around the current index
            animated_sprite.index = (animated_sprite.index + 1) % animated_sprite.frames.len();
        }

        // Set the atlas sprite to match the current frame of the animated sprite
        atlas_sprite.index = animated_sprite.frames[animated_sprite.index];
    }
}

/// System for updating [`AnimatedSprite`]s based on thier [`AnimationBankSprite`]
pub fn update_animation_banks(
    entities: Res<Entities>,
    mut animation_bank_sprites: CompMut<AnimationBankSprite>,
    mut animated_sprites: CompMut<AnimatedSprite>,
) {
    for (ent, animation_bank) in entities.iter_with(&mut animation_bank_sprites) {
        // If the animation has chagned
        if animation_bank.current != animation_bank.last_animation {
            // Update the last animation
            animation_bank.last_animation = animation_bank.current;

            // Get the selected animation from the bank
            let animated_sprite = animation_bank
                .animations
                .get(&animation_bank.current)
                .cloned()
                .unwrap_or_else(|| {
                    panic!("Animation `{}` does not exist.", animation_bank.current)
                });

            // Update the animated sprite with the selected animation
            animated_sprites.insert(ent, animated_sprite);
        }
    }
}
