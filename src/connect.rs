use std::net::Ipv4Addr;
use std::collections::HashMap;
use reqwest::Client;
use url_builder::{url_builder, Part};

use crate::Error;
use crate::DIRIGERA_API_VERSION;
use crate::DIRIGERA_PORT;

#[derive(Debug, Clone)]
pub struct Connect {
    client: Client,
    ip_addr: Ipv4Addr,
    code: String,
    code_verifier: String,
}

impl Connect {
    /// Request a code challenge with the Dirigera device
    /// on your network.
    /// Will be asked to click the button on the Dirigera device.
    pub async fn new(ip_addr: Ipv4Addr) -> Result<Self, Error> {
        let client = Client::builder()
            .danger_accept_invalid_certs(true)
            .user_agent(crate::user_agent())
            .build()?;

        let code_verify = pkce::code_verifier(128);

        let response = client
            .get({
                url_builder! {
                    Part::Scheme("https");
                    Part::HostIpv4(ip_addr);
                    Part::Port(DIRIGERA_PORT);
                    Part::PathSlice(&[
                        DIRIGERA_API_VERSION,
                        "oauth",
                        "authorize"
                    ]);
                    Part::Query(&[
                        ("audience", "homesmart.local"),
                        ("response_type", "code"),
                        ("code_challenge", {
                            pkce::code_challenge(&code_verify).as_str()
                        }),
                        ("code_challenge_method", "S256"),
                    ]);
                }?
            })
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        let code = response
            .get("code")
            .ok_or(Error::CodeNotFound)?
            .to_string();

        Ok(Self {
            client,
            ip_addr,
            code,
            code_verifier: {
                String::from_utf8_lossy(&code_verify).into()
            },
        })
    }

    /// Verify the code is correct.
    pub async fn verify(&self) -> Result<String, Error> {
        let params = HashMap::from([
            ("code", self.code.as_str()),
            ("name", "localhost"),
            ("grant_type", "authorization_code"),
            ("code_verifier", self.code_verifier.as_str()),
        ]);

        let resp = self.client
            .post({
                url_builder! {
                    Part::Scheme("https");
                    Part::HostIpv4(self.ip_addr);
                    Part::Port(DIRIGERA_PORT);
                    Part::PathSlice(&[
                        DIRIGERA_API_VERSION,
                        "oauth",
                        "token"
                    ]);
                    
                }?
            })
            .json(&params)
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        resp
            .get("access_token")
            .map(|x| x.to_string())
            .ok_or(Error::TokenNotFound)
    }
}

