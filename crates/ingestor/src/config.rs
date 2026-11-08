#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GliderNetConfig {
    pub host: String,
    pub port: u16,
    pub filter: String,
}
