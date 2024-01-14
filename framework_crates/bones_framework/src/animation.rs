//! Animation utilities and systems.

use crate::prelude::*;

/// Install animation utilities into the given [`SystemStages`].
pub fn animation_plugin(core: &mut Session) {
    core.stages
        .add_system_to_stage(CoreStage::Last, update_animation_banks)
        .add_system_to_stage(CoreStage::Last, animate_sprites);
}

/// Component that may be added to entities with an [`AtlasSprite`] to animate them.
#[derive(Clone, HasSchema, Debug)]
#[repr(C)]
pub struct AnimatedSprite {
    /// The current frame in the animation.
    pub index: u32,
    /// The frames in the animation.
    ///
    /// These are the indexes into the atlas, specified in the order they will be played.
    // TODO: Put Animation Frames in an `Arc` to Avoid Snapshot Clone Cost.
    pub frames: SVec<u32>,
    /// The frames per second to play the animation at.
    pub fps: f32,
    /// The amount of time the current frame has been playing
    pub timer: f32,
    /// Whether or not to repeat the animation
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
pub struct AnimationBankSprite {
    /// The current animation.
    pub current: Ustr,
    /// The collection of animations in this animation bank.
    // TODO: Put Animation Frames in an `Arc` to Avoid Snapshot Clone Cost.
    // TODO: Use more economic key type such as `ustr`
    pub animations: SMap<Ustr, AnimatedSprite>,
    #[cfg_attr(feature = "serde", serde(default))]
    /// The last animation that was playing.
    pub last_animation: Ustr,
}

impl Default for AnimatedSprite {
    fn default() -> Self {
        Self {
            index: 0,
            frames: default(),
            fps: 0.0,
            timer: 0.0,
            repeat: true,
        }
    }
}

/// System for automatically animating sprites with the [`AnimatedSprite`] component.
pub fn animate_sprites(
    time: Res<Time>,
    entities: Res<Entities>,
    mut atlas_sprites: CompMut<AtlasSprite>,
    mut animated_sprites: CompMut<AnimatedSprite>,
) {
    for (_ent, (atlas_sprite, animated_sprite)) in
        entities.iter_with((&mut atlas_sprites, &mut animated_sprites))
    {
        if animated_sprite.frames.is_empty() {
            continue;
        }

        animated_sprite.timer += time.delta_seconds();

        // If we are ready to go to the next frame
        if (animated_sprite.index != animated_sprite.frames.len() as u32 - 1
            || animated_sprite.repeat)
            && animated_sprite.timer > 1.0 / animated_sprite.fps.max(f32::MIN_POSITIVE)
        {
            // Restart the timer
            animated_sprite.timer = 0.0;

            // Increment and loop around the current index
            animated_sprite.index =
                (animated_sprite.index + 1) % animated_sprite.frames.len() as u32;
        }

        // Set the atlas sprite to match the current frame of the animated sprite
        atlas_sprite.index = animated_sprite.frames[animated_sprite.index as usize];
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
