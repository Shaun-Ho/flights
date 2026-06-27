#[derive(serde::Deserialize)]
pub struct GliderNetConfig {
    pub host: String,
    pub port: u16,
    pub filter: String,
}
