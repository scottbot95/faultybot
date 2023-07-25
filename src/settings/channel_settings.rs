use serde::Deserialize;

#[derive(Default, Debug, Deserialize)]
pub struct ChatSettings {
    pub cooldown: f32,
}

#[derive(Default, Debug, Deserialize)]
pub struct Settings {
    pub chat: ChatSettings,
}
