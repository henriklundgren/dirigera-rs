use std::net::Ipv4Addr;
use serde::Deserialize;

/// If you want to read the configuration from a `toml` file, the [`Config`] is used to deserialize
/// the file contents. It's only available behind the `config` feature flag.
#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    pub ip_address: Ipv4Addr,
    pub token: String,
}

