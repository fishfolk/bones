use bones_reflect_macros::HasTypeRegistration;

/// Example of an airplane asset.
#[derive(HasTypeRegistration)]
pub struct GameMeta {
    pub title: String,
    pub info: GameInfo,
}

#[derive(HasTypeRegistration)]
pub struct GameInfo {
    pub description: String,
    pub authors: Vec<String>,
}

fn main() {}
