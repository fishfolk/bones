use bones_framework::glam::Vec2;
use bones_framework::prelude::{self as bones};
use winit::event::{MouseButton, MouseScrollDelta};
use winit::{event::ElementState, keyboard::KeyCode};

pub trait IntoBones<T> {
    fn into_bones(self) -> T;
}

impl IntoBones<bones::MouseScrollEvent> for MouseScrollDelta {
    fn into_bones(self) -> bones::MouseScrollEvent {
        match self {
            MouseScrollDelta::LineDelta(x, y) => bones::MouseScrollEvent {
                movement: Vec2::new(x, y),
                unit: bones::MouseScrollUnit::Lines,
            },
            MouseScrollDelta::PixelDelta(physical_position) => bones::MouseScrollEvent {
                movement: Vec2::new(physical_position.x as f32, physical_position.y as f32),
                unit: bones::MouseScrollUnit::Pixels,
            },
        }
    }
}

impl IntoBones<bones::ButtonState> for ElementState {
    fn into_bones(self) -> bones::ButtonState {
        match self {
            ElementState::Pressed => bones::ButtonState::Pressed,
            ElementState::Released => bones::ButtonState::Released,
        }
    }
}

impl IntoBones<bones::MouseButton> for MouseButton {
    fn into_bones(self) -> bones::MouseButton {
        match self {
            MouseButton::Left => bones::MouseButton::Left,
            MouseButton::Right => bones::MouseButton::Right,
            MouseButton::Middle => bones::MouseButton::Middle,
            MouseButton::Back => bones::MouseButton::Back,
            MouseButton::Forward => bones::MouseButton::Forward,
            MouseButton::Other(i) => bones::MouseButton::Other(i),
        }
    }
}

impl IntoBones<bones::KeyCode> for winit::keyboard::KeyCode {
    fn into_bones(self) -> bones::KeyCode {
        match self {
            KeyCode::Digit1 => bones::KeyCode::Key1,
            KeyCode::Digit2 => bones::KeyCode::Key2,
            KeyCode::Digit3 => bones::KeyCode::Key3,
            KeyCode::Digit4 => bones::KeyCode::Key4,
            KeyCode::Digit5 => bones::KeyCode::Key5,
            KeyCode::Digit6 => bones::KeyCode::Key6,
            KeyCode::Digit7 => bones::KeyCode::Key7,
            KeyCode::Digit8 => bones::KeyCode::Key8,
            KeyCode::Digit9 => bones::KeyCode::Key9,
            KeyCode::Digit0 => bones::KeyCode::Key0,
            KeyCode::KeyA => bones::KeyCode::A,
            KeyCode::KeyB => bones::KeyCode::B,
            KeyCode::KeyC => bones::KeyCode::C,
            KeyCode::KeyD => bones::KeyCode::D,
            KeyCode::KeyE => bones::KeyCode::E,
            KeyCode::KeyF => bones::KeyCode::F,
            KeyCode::KeyG => bones::KeyCode::G,
            KeyCode::KeyH => bones::KeyCode::H,
            KeyCode::KeyI => bones::KeyCode::I,
            KeyCode::KeyJ => bones::KeyCode::J,
            KeyCode::KeyK => bones::KeyCode::K,
            KeyCode::KeyL => bones::KeyCode::L,
            KeyCode::KeyM => bones::KeyCode::M,
            KeyCode::KeyN => bones::KeyCode::N,
            KeyCode::KeyO => bones::KeyCode::O,
            KeyCode::KeyP => bones::KeyCode::P,
            KeyCode::KeyQ => bones::KeyCode::Q,
            KeyCode::KeyR => bones::KeyCode::R,
            KeyCode::KeyS => bones::KeyCode::S,
            KeyCode::KeyT => bones::KeyCode::T,
            KeyCode::KeyU => bones::KeyCode::U,
            KeyCode::KeyV => bones::KeyCode::V,
            KeyCode::KeyW => bones::KeyCode::W,
            KeyCode::KeyX => bones::KeyCode::X,
            KeyCode::KeyY => bones::KeyCode::Y,
            KeyCode::KeyZ => bones::KeyCode::Z,
            KeyCode::Escape => bones::KeyCode::Escape,
            KeyCode::F1 => bones::KeyCode::F1,
            KeyCode::F2 => bones::KeyCode::F2,
            KeyCode::F3 => bones::KeyCode::F3,
            KeyCode::F4 => bones::KeyCode::F4,
            KeyCode::F5 => bones::KeyCode::F5,
            KeyCode::F6 => bones::KeyCode::F6,
            KeyCode::F7 => bones::KeyCode::F7,
            KeyCode::F8 => bones::KeyCode::F8,
            KeyCode::F9 => bones::KeyCode::F9,
            KeyCode::F10 => bones::KeyCode::F10,
            KeyCode::F11 => bones::KeyCode::F11,
            KeyCode::F12 => bones::KeyCode::F12,
            KeyCode::F13 => bones::KeyCode::F13,
            KeyCode::F14 => bones::KeyCode::F14,
            KeyCode::F15 => bones::KeyCode::F15,
            KeyCode::F16 => bones::KeyCode::F16,
            KeyCode::F17 => bones::KeyCode::F17,
            KeyCode::F18 => bones::KeyCode::F18,
            KeyCode::F19 => bones::KeyCode::F19,
            KeyCode::F20 => bones::KeyCode::F20,
            KeyCode::F21 => bones::KeyCode::F21,
            KeyCode::F22 => bones::KeyCode::F22,
            KeyCode::F23 => bones::KeyCode::F23,
            KeyCode::F24 => bones::KeyCode::F24,
            KeyCode::PrintScreen => bones::KeyCode::Snapshot,
            // Normally on the same key as PrintScreen, and we are already mapping it
            //KeyCode::PrintScreen => bones::KeyCode::Sysrq,
            KeyCode::ScrollLock => bones::KeyCode::Scroll,
            KeyCode::Pause => bones::KeyCode::Pause,
            KeyCode::Insert => bones::KeyCode::Insert,
            KeyCode::Home => bones::KeyCode::Home,
            KeyCode::Delete => bones::KeyCode::Delete,
            KeyCode::End => bones::KeyCode::End,
            KeyCode::PageDown => bones::KeyCode::PageDown,
            KeyCode::PageUp => bones::KeyCode::PageUp,
            KeyCode::ArrowLeft => bones::KeyCode::Left,
            KeyCode::ArrowUp => bones::KeyCode::Up,
            KeyCode::ArrowRight => bones::KeyCode::Right,
            KeyCode::ArrowDown => bones::KeyCode::Down,
            KeyCode::Backspace => bones::KeyCode::Back,
            KeyCode::Enter => bones::KeyCode::Return,
            KeyCode::Space => bones::KeyCode::Space,
            KeyCode::NumLock => bones::KeyCode::Numlock,
            KeyCode::Numpad0 => bones::KeyCode::Numpad0,
            KeyCode::Numpad1 => bones::KeyCode::Numpad1,
            KeyCode::Numpad2 => bones::KeyCode::Numpad2,
            KeyCode::Numpad3 => bones::KeyCode::Numpad3,
            KeyCode::Numpad4 => bones::KeyCode::Numpad4,
            KeyCode::Numpad5 => bones::KeyCode::Numpad5,
            KeyCode::Numpad6 => bones::KeyCode::Numpad6,
            KeyCode::Numpad7 => bones::KeyCode::Numpad7,
            KeyCode::Numpad8 => bones::KeyCode::Numpad8,
            KeyCode::Numpad9 => bones::KeyCode::Numpad9,
            KeyCode::NumpadAdd => bones::KeyCode::NumpadAdd,
            KeyCode::Equal => bones::KeyCode::Equals,
            // Winit doesn't differentiate both '+' and '=', considering they are usually
            // on the same physical key, and we are already mapping it
            //KeyCode::Equal => bones::KeyCode::Plus,
            KeyCode::Backslash => bones::KeyCode::Backslash,
            // LaunchApp2 is sometimes named Calculator
            KeyCode::LaunchApp2 => bones::KeyCode::Calculator,
            KeyCode::CapsLock => bones::KeyCode::Capital,
            KeyCode::Comma => bones::KeyCode::Comma,
            KeyCode::Convert => bones::KeyCode::Convert,
            KeyCode::NumpadDecimal => bones::KeyCode::NumpadDecimal,
            KeyCode::NumpadDivide => bones::KeyCode::NumpadDivide,
            KeyCode::Backquote => bones::KeyCode::Grave,
            KeyCode::AltLeft => bones::KeyCode::AltLeft,
            KeyCode::BracketLeft => bones::KeyCode::BracketLeft,
            KeyCode::ControlLeft => bones::KeyCode::ControlLeft,
            KeyCode::ShiftLeft => bones::KeyCode::ShiftLeft,
            KeyCode::SuperLeft => bones::KeyCode::SuperLeft,
            KeyCode::LaunchMail => bones::KeyCode::Mail,
            KeyCode::MediaSelect => bones::KeyCode::MediaSelect,
            KeyCode::MediaStop => bones::KeyCode::MediaStop,
            KeyCode::Minus => bones::KeyCode::Minus,
            KeyCode::NumpadMultiply => bones::KeyCode::NumpadMultiply,
            KeyCode::AudioVolumeMute => bones::KeyCode::Mute,
            // LaunchApp1 is sometimes named MyComputer
            KeyCode::LaunchApp1 => bones::KeyCode::MyComputer,
            KeyCode::MediaTrackNext => bones::KeyCode::NextTrack,
            KeyCode::NumpadComma => bones::KeyCode::NumpadComma,
            KeyCode::NumpadEnter => bones::KeyCode::NumpadEnter,
            KeyCode::NumpadEqual => bones::KeyCode::NumpadEquals,
            KeyCode::Period => bones::KeyCode::Period,
            KeyCode::MediaPlayPause => bones::KeyCode::PlayPause,
            KeyCode::Power => bones::KeyCode::Power,
            KeyCode::MediaTrackPrevious => bones::KeyCode::PrevTrack,
            KeyCode::AltRight => bones::KeyCode::AltRight,
            KeyCode::BracketRight => bones::KeyCode::BracketRight,
            KeyCode::ControlRight => bones::KeyCode::ControlRight,
            KeyCode::ShiftRight => bones::KeyCode::ShiftRight,
            KeyCode::SuperRight => bones::KeyCode::SuperRight,
            KeyCode::Semicolon => bones::KeyCode::Semicolon,
            KeyCode::Slash => bones::KeyCode::Slash,
            KeyCode::Sleep => bones::KeyCode::Sleep,
            KeyCode::NumpadSubtract => bones::KeyCode::NumpadSubtract,
            KeyCode::Tab => bones::KeyCode::Tab,
            KeyCode::AudioVolumeDown => bones::KeyCode::VolumeDown,
            KeyCode::AudioVolumeUp => bones::KeyCode::VolumeUp,
            KeyCode::WakeUp => bones::KeyCode::Wake,
            KeyCode::BrowserBack => bones::KeyCode::WebBack,
            KeyCode::BrowserFavorites => bones::KeyCode::WebFavorites,
            KeyCode::BrowserForward => bones::KeyCode::WebForward,
            KeyCode::BrowserHome => bones::KeyCode::WebHome,
            KeyCode::BrowserRefresh => bones::KeyCode::WebRefresh,
            KeyCode::BrowserSearch => bones::KeyCode::WebSearch,
            KeyCode::BrowserStop => bones::KeyCode::WebStop,
            KeyCode::IntlYen => bones::KeyCode::Yen,
            KeyCode::Copy => bones::KeyCode::Copy,
            KeyCode::Paste => bones::KeyCode::Paste,
            KeyCode::Cut => bones::KeyCode::Cut,
            // These are not on the latest winit version,
            // need to figure out how to handle them.
            //KeyCode:: => bones::KeyCode::AbntC1,
            //KeyCode:: => bones::KeyCode::AbntC2,
            //KeyCode:: => bones::KeyCode::Apostrophe,
            //KeyCode:: => bones::KeyCode::Asterisk,

            // Not sure how to implement this, maybe NonConvert?
            //KeyCode::NonConvert => bones::KeyCode::NoConvert,

            // Not sure how to implement this
            //KeyCode:: => bones::KeyCode::At,
            //KeyCode:: => bones::KeyCode::Ax,
            //KeyCode:: => bones::KeyCode::Colon,
            //KeyCode:: => bones::KeyCode::Kana,
            //KeyCode:: => bones::KeyCode::Kanji,
            //KeyCode:: => bones::KeyCode::Unlabeled,
            //KeyCode:: => bones::KeyCode::Oem102,
            //KeyCode:: => bones::KeyCode::Underline,
            //KeyCode:: => bones::KeyCode::Stop,

            // I think this is the same as PageUp and PageDown keys, but not sure.
            // tho they are already mapped
            //KeyCode::PageUp => bones::KeyCode::NavigateForward,
            //KeyCode::PageDown => bones::KeyCode::NavigateBackward,

            // These are named keys now on winit, need to
            // figure out how to handle them.
            //NamedKey::Compose => bones::KeyCode::Compose,
            //NamedKey::Caret => bones::KeyCode::Caret,

            // Not sure what this is, could be MediaApps or ContextMenu, or both
            //KeyCode::MediaApps | KeyCode::ContextMenu => bones::KeyCode::Apps,
            _ => unimplemented!(),
        }
    }
}
