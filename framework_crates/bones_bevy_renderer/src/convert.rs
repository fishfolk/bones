use bevy::{
    input::{mouse::MouseScrollUnit, ButtonState},
    prelude::*,
    render::camera::Viewport,
};
use bones_framework::prelude as bones;

pub trait IntoBevy<T> {
    fn into_bevy(self) -> T;
}
pub trait IntoBones<T> {
    fn into_bones(self) -> T;
}

impl IntoBevy<Viewport> for bones::Viewport {
    fn into_bevy(self) -> Viewport {
        Viewport {
            physical_position: self.position,
            physical_size: self.size,
            depth: self.depth_min..self.depth_max,
        }
    }
}

impl IntoBevy<Color> for bones::Color {
    fn into_bevy(self) -> Color {
        Color::Rgba {
            red: self.r(),
            green: self.g(),
            blue: self.b(),
            alpha: self.a(),
        }
    }
}

impl IntoBevy<Transform> for bones::Transform {
    fn into_bevy(self) -> Transform {
        Transform {
            translation: self.translation,
            rotation: self.rotation,
            scale: self.scale,
        }
    }
}

impl IntoBones<bones::MouseScrollUnit> for MouseScrollUnit {
    fn into_bones(self) -> bones::MouseScrollUnit {
        match self {
            MouseScrollUnit::Line => bones::MouseScrollUnit::Lines,
            MouseScrollUnit::Pixel => bones::MouseScrollUnit::Pixels,
        }
    }
}

impl IntoBones<bones::ButtonState> for ButtonState {
    fn into_bones(self) -> bones::ButtonState {
        match self {
            ButtonState::Pressed => bones::ButtonState::Pressed,
            ButtonState::Released => bones::ButtonState::Released,
        }
    }
}

impl IntoBones<bones::MouseButton> for MouseButton {
    fn into_bones(self) -> bones::MouseButton {
        match self {
            MouseButton::Left => bones::MouseButton::Left,
            MouseButton::Right => bones::MouseButton::Right,
            MouseButton::Middle => bones::MouseButton::Middle,
            MouseButton::Other(i) => bones::MouseButton::Other(i),
        }
    }
}

impl IntoBones<bones::KeyCode> for KeyCode {
    fn into_bones(self) -> bones::KeyCode {
        match self {
            KeyCode::Key1 => bones::KeyCode::Key1,
            KeyCode::Key2 => bones::KeyCode::Key2,
            KeyCode::Key3 => bones::KeyCode::Key3,
            KeyCode::Key4 => bones::KeyCode::Key4,
            KeyCode::Key5 => bones::KeyCode::Key5,
            KeyCode::Key6 => bones::KeyCode::Key6,
            KeyCode::Key7 => bones::KeyCode::Key7,
            KeyCode::Key8 => bones::KeyCode::Key8,
            KeyCode::Key9 => bones::KeyCode::Key9,
            KeyCode::Key0 => bones::KeyCode::Key0,
            KeyCode::A => bones::KeyCode::A,
            KeyCode::B => bones::KeyCode::B,
            KeyCode::C => bones::KeyCode::C,
            KeyCode::D => bones::KeyCode::D,
            KeyCode::E => bones::KeyCode::E,
            KeyCode::F => bones::KeyCode::F,
            KeyCode::G => bones::KeyCode::G,
            KeyCode::H => bones::KeyCode::H,
            KeyCode::I => bones::KeyCode::I,
            KeyCode::J => bones::KeyCode::J,
            KeyCode::K => bones::KeyCode::K,
            KeyCode::L => bones::KeyCode::L,
            KeyCode::M => bones::KeyCode::M,
            KeyCode::N => bones::KeyCode::N,
            KeyCode::O => bones::KeyCode::O,
            KeyCode::P => bones::KeyCode::P,
            KeyCode::Q => bones::KeyCode::Q,
            KeyCode::R => bones::KeyCode::R,
            KeyCode::S => bones::KeyCode::S,
            KeyCode::T => bones::KeyCode::T,
            KeyCode::U => bones::KeyCode::U,
            KeyCode::V => bones::KeyCode::V,
            KeyCode::W => bones::KeyCode::W,
            KeyCode::X => bones::KeyCode::X,
            KeyCode::Y => bones::KeyCode::Y,
            KeyCode::Z => bones::KeyCode::Z,
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
            KeyCode::Snapshot => bones::KeyCode::Snapshot,
            KeyCode::Scroll => bones::KeyCode::Scroll,
            KeyCode::Pause => bones::KeyCode::Pause,
            KeyCode::Insert => bones::KeyCode::Insert,
            KeyCode::Home => bones::KeyCode::Home,
            KeyCode::Delete => bones::KeyCode::Delete,
            KeyCode::End => bones::KeyCode::End,
            KeyCode::PageDown => bones::KeyCode::PageDown,
            KeyCode::PageUp => bones::KeyCode::PageUp,
            KeyCode::Left => bones::KeyCode::Left,
            KeyCode::Up => bones::KeyCode::Up,
            KeyCode::Right => bones::KeyCode::Right,
            KeyCode::Down => bones::KeyCode::Down,
            KeyCode::Back => bones::KeyCode::Back,
            KeyCode::Return => bones::KeyCode::Return,
            KeyCode::Space => bones::KeyCode::Space,
            KeyCode::Compose => bones::KeyCode::Compose,
            KeyCode::Caret => bones::KeyCode::Caret,
            KeyCode::Numlock => bones::KeyCode::Numlock,
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
            KeyCode::AbntC1 => bones::KeyCode::AbntC1,
            KeyCode::AbntC2 => bones::KeyCode::AbntC2,
            KeyCode::NumpadAdd => bones::KeyCode::NumpadAdd,
            KeyCode::Apostrophe => bones::KeyCode::Apostrophe,
            KeyCode::Apps => bones::KeyCode::Apps,
            KeyCode::Asterisk => bones::KeyCode::Asterisk,
            KeyCode::Plus => bones::KeyCode::Plus,
            KeyCode::At => bones::KeyCode::At,
            KeyCode::Ax => bones::KeyCode::Ax,
            KeyCode::Backslash => bones::KeyCode::Backslash,
            KeyCode::Calculator => bones::KeyCode::Calculator,
            KeyCode::Capital => bones::KeyCode::Capital,
            KeyCode::Colon => bones::KeyCode::Colon,
            KeyCode::Comma => bones::KeyCode::Comma,
            KeyCode::Convert => bones::KeyCode::Convert,
            KeyCode::NumpadDecimal => bones::KeyCode::NumpadDecimal,
            KeyCode::NumpadDivide => bones::KeyCode::NumpadDivide,
            KeyCode::Equals => bones::KeyCode::Equals,
            KeyCode::Grave => bones::KeyCode::Grave,
            KeyCode::Kana => bones::KeyCode::Kana,
            KeyCode::Kanji => bones::KeyCode::Kanji,
            KeyCode::AltLeft => bones::KeyCode::AltLeft,
            KeyCode::BracketLeft => bones::KeyCode::BracketLeft,
            KeyCode::ControlLeft => bones::KeyCode::ControlLeft,
            KeyCode::ShiftLeft => bones::KeyCode::ShiftLeft,
            KeyCode::SuperLeft => bones::KeyCode::SuperLeft,
            KeyCode::Mail => bones::KeyCode::Mail,
            KeyCode::MediaSelect => bones::KeyCode::MediaSelect,
            KeyCode::MediaStop => bones::KeyCode::MediaStop,
            KeyCode::Minus => bones::KeyCode::Minus,
            KeyCode::NumpadMultiply => bones::KeyCode::NumpadMultiply,
            KeyCode::Mute => bones::KeyCode::Mute,
            KeyCode::MyComputer => bones::KeyCode::MyComputer,
            KeyCode::NavigateForward => bones::KeyCode::NavigateForward,
            KeyCode::NavigateBackward => bones::KeyCode::NavigateBackward,
            KeyCode::NextTrack => bones::KeyCode::NextTrack,
            KeyCode::NoConvert => bones::KeyCode::NoConvert,
            KeyCode::NumpadComma => bones::KeyCode::NumpadComma,
            KeyCode::NumpadEnter => bones::KeyCode::NumpadEnter,
            KeyCode::NumpadEquals => bones::KeyCode::NumpadEquals,
            KeyCode::Oem102 => bones::KeyCode::Oem102,
            KeyCode::Period => bones::KeyCode::Period,
            KeyCode::PlayPause => bones::KeyCode::PlayPause,
            KeyCode::Power => bones::KeyCode::Power,
            KeyCode::PrevTrack => bones::KeyCode::PrevTrack,
            KeyCode::AltRight => bones::KeyCode::AltRight,
            KeyCode::BracketRight => bones::KeyCode::BracketRight,
            KeyCode::ControlRight => bones::KeyCode::ControlRight,
            KeyCode::ShiftRight => bones::KeyCode::ShiftRight,
            KeyCode::SuperRight => bones::KeyCode::SuperRight,
            KeyCode::Semicolon => bones::KeyCode::Semicolon,
            KeyCode::Slash => bones::KeyCode::Slash,
            KeyCode::Sleep => bones::KeyCode::Sleep,
            KeyCode::Stop => bones::KeyCode::Stop,
            KeyCode::NumpadSubtract => bones::KeyCode::NumpadSubtract,
            KeyCode::Sysrq => bones::KeyCode::Sysrq,
            KeyCode::Tab => bones::KeyCode::Tab,
            KeyCode::Underline => bones::KeyCode::Underline,
            KeyCode::Unlabeled => bones::KeyCode::Unlabeled,
            KeyCode::VolumeDown => bones::KeyCode::VolumeDown,
            KeyCode::VolumeUp => bones::KeyCode::VolumeUp,
            KeyCode::Wake => bones::KeyCode::Wake,
            KeyCode::WebBack => bones::KeyCode::WebBack,
            KeyCode::WebFavorites => bones::KeyCode::WebFavorites,
            KeyCode::WebForward => bones::KeyCode::WebForward,
            KeyCode::WebHome => bones::KeyCode::WebHome,
            KeyCode::WebRefresh => bones::KeyCode::WebRefresh,
            KeyCode::WebSearch => bones::KeyCode::WebSearch,
            KeyCode::WebStop => bones::KeyCode::WebStop,
            KeyCode::Yen => bones::KeyCode::Yen,
            KeyCode::Copy => bones::KeyCode::Copy,
            KeyCode::Paste => bones::KeyCode::Paste,
            KeyCode::Cut => bones::KeyCode::Cut,
        }
    }
}

impl IntoBones<bones::GamepadButton> for bevy::input::gamepad::GamepadButtonType {
    fn into_bones(self) -> bones::GamepadButton {
        match self {
            GamepadButtonType::South => bones::GamepadButton::South,
            GamepadButtonType::East => bones::GamepadButton::East,
            GamepadButtonType::North => bones::GamepadButton::North,
            GamepadButtonType::West => bones::GamepadButton::West,
            GamepadButtonType::C => bones::GamepadButton::C,
            GamepadButtonType::Z => bones::GamepadButton::Z,
            GamepadButtonType::LeftTrigger => bones::GamepadButton::LeftTrigger,
            GamepadButtonType::LeftTrigger2 => bones::GamepadButton::LeftTrigger2,
            GamepadButtonType::RightTrigger => bones::GamepadButton::RightTrigger,
            GamepadButtonType::RightTrigger2 => bones::GamepadButton::RightTrigger2,
            GamepadButtonType::Select => bones::GamepadButton::Select,
            GamepadButtonType::Start => bones::GamepadButton::Start,
            GamepadButtonType::Mode => bones::GamepadButton::Mode,
            GamepadButtonType::LeftThumb => bones::GamepadButton::LeftThumb,
            GamepadButtonType::RightThumb => bones::GamepadButton::RightThumb,
            GamepadButtonType::DPadUp => bones::GamepadButton::DPadUp,
            GamepadButtonType::DPadDown => bones::GamepadButton::DPadDown,
            GamepadButtonType::DPadLeft => bones::GamepadButton::DPadLeft,
            GamepadButtonType::DPadRight => bones::GamepadButton::DPadRight,
            GamepadButtonType::Other(x) => bones::GamepadButton::Other(x),
        }
    }
}

impl IntoBones<bones::GamepadAxis> for bevy::input::gamepad::GamepadAxisType {
    fn into_bones(self) -> bones::GamepadAxis {
        match self {
            GamepadAxisType::LeftStickX => bones::GamepadAxis::LeftStickX,
            GamepadAxisType::LeftStickY => bones::GamepadAxis::LeftStickY,
            GamepadAxisType::LeftZ => bones::GamepadAxis::LeftZ,
            GamepadAxisType::RightStickX => bones::GamepadAxis::RightStickX,
            GamepadAxisType::RightStickY => bones::GamepadAxis::RightStickY,
            GamepadAxisType::RightZ => bones::GamepadAxis::RightZ,
            GamepadAxisType::Other(x) => bones::GamepadAxis::Other(x),
        }
    }
}
