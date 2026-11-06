use std::time::Duration;

use bones_framework::{
    input::gilrs::GILRS_CONTEXT,
    prelude::{GamepadRumbleIntensity, GamepadRumbleRequest, GamepadsRumble, Instant},
};
use gilrs::{
    ff::{BaseEffect, BaseEffectType, Effect, EffectBuilder, Replay, Ticks},
    Gilrs,
};

struct RunningRumble {
    gamepad: u32,
    _effect: Effect,
    deadline: Instant,
}

#[derive(Default)]
pub struct RumbleManager {
    running: Vec<RunningRumble>,
}

impl RumbleManager {
    pub fn process(&mut self, rumble: &mut GamepadsRumble) {
        let now = Instant::now();
        self.running.retain(|r| r.deadline > now);

        if rumble.requests.is_empty() {
            return;
        }

        let mut gilrs = GILRS_CONTEXT.lock().unwrap_or_else(|e| e.into_inner());
        while let Some(request) = rumble.requests.pop_front() {
            match request {
                GamepadRumbleRequest::AddRumble {
                    gamepad,
                    intensity,
                    duration,
                } => {
                    self.add_effect(&mut gilrs, gamepad, intensity, duration, now);
                }
                GamepadRumbleRequest::SetRumble {
                    gamepad,
                    intensity,
                    duration,
                } => {
                    self.running.retain(|r| r.gamepad != gamepad);
                    self.add_effect(&mut gilrs, gamepad, intensity, duration, now);
                }
                GamepadRumbleRequest::Stop { gamepad } => {
                    self.running.retain(|r| r.gamepad != gamepad);
                }
            }
        }
    }

    fn add_effect(
        &mut self,
        gilrs: &mut Gilrs,
        gamepad: u32,
        intensity: GamepadRumbleIntensity,
        duration: f32,
        now: Instant,
    ) {
        if !duration.is_finite()
            || duration <= 0.0
            || (intensity.strong_motor() <= 0.0 && intensity.weak_motor() <= 0.0)
        {
            return;
        }
        let duration = duration.min(3600.0);

        let Some((gamepad_id, _)) = gilrs
            .gamepads()
            .find(|(id, gp)| usize::from(*id) as u32 == gamepad && gp.is_ff_supported())
        else {
            return;
        };

        let duration_ticks = Ticks::from_ms((duration * 1000.0) as u32);
        let scheduling = Replay {
            play_for: duration_ticks,
            ..Default::default()
        };
        let strong = BaseEffect {
            kind: BaseEffectType::Strong {
                magnitude: (intensity.strong_motor().clamp(0.0, 1.0) * u16::MAX as f32) as u16,
            },
            scheduling,
            envelope: Default::default(),
        };
        let weak = BaseEffect {
            kind: BaseEffectType::Weak {
                magnitude: (intensity.weak_motor().clamp(0.0, 1.0) * u16::MAX as f32) as u16,
            },
            scheduling,
            envelope: Default::default(),
        };

        if let Ok(effect) = EffectBuilder::new()
            .add_effect(strong)
            .add_effect(weak)
            .gamepads(&[gamepad_id])
            .finish(gilrs)
        {
            if effect.play().is_ok() {
                self.running.push(RunningRumble {
                    gamepad,
                    _effect: effect,
                    deadline: now + Duration::from_secs_f32(duration),
                });
            }
        }
    }
}
