//! Dirigera: Manger your IKEA devices.
//! Dirigera is a client to communicate with your IKEA Dirigera hub and control your Trådfri
//! devices. ~~It is built with [`hyper`] and is bundled with an optional tool to generate the token
//! you need for the communication.~~
mod device;
mod hub;
pub mod scene;
pub mod traits;
mod connect;
mod config;
mod errors;

pub use hub::Hub;
pub use errors::Error;
pub use config::Config;
pub use connect::Connect;
pub use device::{
    Device,
    DeviceData,
    DeviceType
};
pub use scene::Scene;

use std::sync::OnceLock;
use serde::Deserialize;

pub(crate) const DIRIGERA_PORT: u16 = 8443;
pub(crate) const DIRIGERA_API_VERSION: &str = "v1";

pub(crate) fn user_agent() -> &'static str {
    static USER_AGENT: OnceLock<String> = OnceLock::new();
    USER_AGENT.get_or_init(|| {
        let version = std::env!("CARGO_PKG_VERSION");
        let name = std::env!("CARGO_PKG_NAME");

        format!("{}-rs/{}", name, version)
    })
}

pub(crate) fn deserialize_datetime<'de, D>(
    deserializer: D,
) -> Result<chrono::DateTime<chrono::Utc>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let date_str = String::deserialize(deserializer)?;
    match date_str.parse() {
        Ok(system_time) => Ok(system_time),
        Err(_) => Err(serde::de::Error::custom("Invalid date format")),
    }
}

pub(crate) fn deserialize_datetime_optional<'de, D>(
    deserializer: D,
) -> Result<Option<chrono::DateTime<chrono::Utc>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    match deserialize_datetime(deserializer) {
        Ok(system_time) => Ok(Some(system_time)),
        Err(_) => Ok(None),
    }
}

