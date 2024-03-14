//! The IKEA hub is what you're communicating with and what's running the API to manage your
//! devices. Because of that the [`Hub`] is what's exposing all methods from the API. The API is a
//! RESTful HTTPS API with a self signed certificate so you need a [`hyper`] client that doesn't do
//! TLS verification. You also need a bearer token which is obtain via OAuth 2. Configuration for
//! TLS and tool to get a token is both available under the [`danger`](crate::danger) module and the
//! `config` feature flag respectively.
use std::net::Ipv4Addr;
use std::collections::HashMap;
use reqwest::Client;
use reqwest::header::{
    HeaderMap,
    HeaderValue,
    AUTHORIZATION,
    CONTENT_TYPE,
};
use url_builder::{url_builder, Part};

use crate::Device;
use crate::DIRIGERA_API_VERSION;
use crate::DIRIGERA_PORT;
use crate::traits::DirigeraExt;
use crate::config::Config;

/// A [`Hub`] consists of a [`reqwest`] client, and the hub's IP address to communicate with
/// it.
#[derive(Debug, Clone)]
pub struct Hub {
    client: Client,
    ip_address: Ipv4Addr,
}

#[async_trait::async_trait]
impl DirigeraExt for Hub {
    type Rejection = crate::Error;
    type Config = Config;
    type Device = Device;

    fn new(config: &Self::Config) -> Result<Self, Self::Rejection> {
        // base_url PR https://github.com/seanmonstar/reqwest/pull/1620
        let client = Client::builder()
            .danger_accept_invalid_certs(true)
            .user_agent(crate::user_agent())
            .default_headers({
                let mut headers = HeaderMap::new();
                let bearer_token = format!("Bearer {}", config.token);
                let mut auth_value = HeaderValue::from_str(&bearer_token)?;
                auth_value.set_sensitive(true);
                headers.insert(AUTHORIZATION, auth_value);
                let content_type = HeaderValue::from_static("application/json");
                headers.insert(CONTENT_TYPE, content_type);

                headers
            })
            .build()?;

        Ok(Self {
            client,
            ip_address: config.ip_address,
        })
    }

    /// List all devices that is known for the [`Hub`]. This will return an exhaustive list of
    /// [`Device`](crate::Device)s.
    async fn list(&self) -> Result<Vec<Self::Device>, Self::Rejection> {
        self.client
            .get({
                url_builder! {
                    Part::Scheme("https");
                    Part::HostIpv4(self.ip_address);
                    Part::Port(DIRIGERA_PORT);
                    Part::PathSlice(&[
                        DIRIGERA_API_VERSION,
                        "devices",
                    ])
                }?
            })
            .send()
            .await?
            .json::<Vec<Device>>()
            .await
            .map_err(Self::Rejection::BuildError)
    }

    /// Get a single [`Device`](crate::Device) based on its id.
    async fn get(&self, id: &str) -> Result<Self::Device, Self::Rejection> {
        self.client
            .get({
                url_builder! {
                    Part::Scheme("https");
                    Part::HostIpv4(self.ip_address);
                    Part::Port(DIRIGERA_PORT);
                    Part::PathSlice(&[
                        DIRIGERA_API_VERSION,
                        "devices",
                        id,
                    ])
                }?
            })
            .send()
            .await?
            .json::<Device>()
            .await
            .map_err(Self::Rejection::BuildError)
    }

}

impl Hub {
    /// Rename a [`Device`](crate::Device). The function takes a mutable reference to the
    /// [`Device`](crate::Device) because on successful renaming the passed
    /// [`Device`](crate::Device) will be updated with the new name.
    pub async fn rename(
        &mut self,
        device: &mut crate::device::Device,
        new_name: &str,
    ) -> anyhow::Result<()> {
        let inner = device.inner_mut();

        if !has_capability(
            inner.capabilities.can_receive.as_ref(),
            &[crate::device::Capability::CustomName],
        ) {
            anyhow::bail!("device cannot change name");
        }

        let mut attributes = HashMap::new();
        attributes.insert("customName", new_name);

        let mut body = HashMap::new();
        body.insert("attributes", attributes);

        let body: String = serde_json::to_string(&vec![body])?;

        self.client
            .patch({
                make_url(self.ip_address, &format!("/devices/{}", inner.id))?
            })
            .body(body)
            .send()
            .await
            .map_err(|err| anyhow::anyhow!(err))?;

        inner.attributes.custom_name = new_name.to_string();

        Ok(())
    }

    /// Toggle a [`Device`](crate::Device) on and off. Requires the [`Device`](crate::Device) to
    /// support [`Capability::IsOn`](crate::device::Capability::IsOn) as a receivable capability.
    /// The function takes a mutable reference to the [`Device`](crate::Device) because on
    /// successful toggle the passed
    /// [`Device`](crate::Device) will be updated with the new state.
    pub async fn toggle_on_off(
        &mut self,
        device: &mut crate::device::Device,
    ) -> anyhow::Result<()> {
        let inner = device.inner_mut();

        if !has_capability(
            inner.capabilities.can_receive.as_ref(),
            &[crate::device::Capability::IsOn],
        ) {
            anyhow::bail!("device cannot be toggled");
        }

        let mut attributes = HashMap::new();
        inner
            .attributes
            .is_on
            .map(|x| attributes.insert("isOn", !x));

        let mut body = HashMap::new();
        body.insert("attributes", attributes);

        let body: String = serde_json::to_string(&vec![body])?;

        self.client
            .patch({
                make_url(self.ip_address, &format!("/devices/{}", inner.id))?
            })
            .body(body)
            .send()
            .await
            .map_err(|err| anyhow::anyhow!(err))?;

        inner.attributes.is_on = inner.attributes.is_on.map(|x| !x);

        Ok(())
    }

    /// Set light level on the [`Device`](crate::Device). Requires the [`Device`](crate::Device) to
    /// support [`Capability::LightLevel`](crate::device::Capability::LightLevel) as a receivable
    /// capability. The function takes a mutable reference to the [`Device`](crate::Device) because
    /// on successful change the passed [`Device`](crate::Device) will be updated with the new
    /// light level.
    pub async fn set_light_level(
        &mut self,
        device: &mut crate::device::Device,
        level: u8,
    ) -> anyhow::Result<()> {
        let inner = device.inner_mut();

        if !has_capability(
            inner.capabilities.can_receive.as_ref(),
            &[crate::device::Capability::LightLevel],
        ) {
            anyhow::bail!("device cannot set light level");
        }

        if level > 100 {
            anyhow::bail!("level must be between 0.0 -> 100.0");
        }

        let mut attributes = HashMap::new();
        attributes.insert("lightLevel", level);

        let mut body = HashMap::new();
        body.insert("attributes", attributes);

        let body: String = serde_json::to_string(&vec![body])?;

        self.client
            .patch({
                make_url(self.ip_address, &format!("/devices/{}", inner.id))?
            })
            .body(body)
            .send()
            .await
            .map_err(|err| anyhow::anyhow!(err))?;

        inner.attributes.light_level = Some(level);

        Ok(())
    }

    /// Set color temperature on the [`Device`](crate::Device). Requires the
    /// [`Device`](crate::Device) to support
    /// [`Capability::ColorTemperature`](crate::device::Capability::ColorTemperature) as a
    /// receivable capability. The function takes a mutable reference to the
    /// [`Device`](crate::Device) because on successful change the passed [`Device`](crate::Device)
    /// will be updated with the new color temperature.
    pub async fn set_temperature(
        &mut self,
        device: &mut crate::device::Device,
        temperature: u16,
    ) -> anyhow::Result<()> {
        let inner = device.inner_mut();

        if !has_capability(
            inner.capabilities.can_receive.as_ref(),
            &[crate::device::Capability::ColorTemperature],
        ) {
            anyhow::bail!("device cannot set color temperature");
        }

        let min = inner
            .attributes
            .color_temperature_min
            .ok_or_else(|| anyhow::anyhow!("device has no min temperature value"))?;
        let max = inner
            .attributes
            .color_temperature_max
            .ok_or_else(|| anyhow::anyhow!("device has no max temperature value"))?;

        if !(max..=min).contains(&temperature) {
            anyhow::bail!("color temperature {temperature} not within {min} -> {max}");
        }

        let mut attributes = HashMap::new();
        attributes.insert("colorTemperature", temperature);

        let mut body = HashMap::new();
        body.insert("attributes", attributes);

        let body: String = serde_json::to_string(&vec![body])?;

        self.client
            .patch({
                make_url(self.ip_address, &format!("/devices/{}", inner.id))?
            })
            .body(body)
            .send()
            .await
            .map_err(|err| anyhow::anyhow!(err))?;

        inner.attributes.color_temperature = Some(temperature);

        Ok(())
    }

    /// Set hue and saturation on the [`Device`](crate::Device). Requires the
    /// [`Device`](crate::Device) to support
    /// [`Capability::ColorHue`](crate::device::Capability::ColorHue) and
    /// [`Capability::ColorSaturation`](crate::device::Capability::ColorSaturation) as a receivable
    /// capability. The function takes a mutable reference to the [`Device`](crate::Device) because
    /// on successful change the passed [`Device`](crate::Device) will be updated with the new hue
    /// and saturation.
    pub async fn set_hue_saturation(
        &mut self,
        device: &mut crate::device::Device,
        hue: f64,
        saturation: f64,
    ) -> anyhow::Result<()> {
        let inner = device.inner_mut();

        if !has_capability(
            inner.capabilities.can_receive.as_ref(),
            &[
                crate::device::Capability::ColorHue,
                crate::device::Capability::ColorSaturation,
            ],
        ) {
            anyhow::bail!("device cannot be change for hue and saturation");
        }

        if !(0f64..=360f64).contains(&hue) {
            anyhow::bail!("hue must be between 0.0 -> 360.0");
        }

        if !(0f64..=1f64).contains(&saturation) {
            anyhow::bail!("hue must be between 0.0 -> 1.0");
        }

        let mut attributes = HashMap::new();
        attributes.insert("colorHue", hue);
        attributes.insert("colorSaturation", saturation);

        let mut body = HashMap::new();
        body.insert("attributes", attributes);

        let body: String = serde_json::to_string(&vec![body])?;

        self.client
            .patch({
                make_url(self.ip_address, &format!("/devices/{}", inner.id))?
            })
            .body(body)
            .send()
            .await
            .map_err(|err| anyhow::anyhow!(err))?;

        inner.attributes.color_hue = Some(hue);
        inner.attributes.color_saturation = Some(hue);

        Ok(())
    }

    /// Set startup behaviour on the [`Device`](crate::Device). The function takes a mutable
    /// reference to the [`Device`](crate::Device) because on successful change the passed
    /// [`Device`](crate::Device) will be updated with the new startup behaviour.
    pub async fn set_startup_behaviour(
        &mut self,
        device: &mut crate::device::Device,
        behaviour: crate::device::Startup,
    ) -> anyhow::Result<()> {
        let inner = device.inner_mut();

        let mut attributes = HashMap::new();
        attributes.insert("startupOnOff", &behaviour);

        let mut body = HashMap::new();
        body.insert("attributes", attributes);

        let body: String = serde_json::to_string(&vec![body])?;

        self.client
            .patch({
                make_url(self.ip_address, &format!("/devices/{}", inner.id))?
            })
            .body(body)
            .send()
            .await
            .map_err(|err| anyhow::anyhow!(err))?;

        inner.attributes.startup_on_off = Some(behaviour);

        Ok(())
    }

    /// Set target level on the [`Device`](crate::Device). Requires the [`Device`](crate::Device)
    /// to support [`Capability::BlindsState`](crate::device::Capability::BlindsState) as a
    /// receivable capability. The function takes a mutable reference to the
    /// [`Device`](crate::Device) because on successful change the passed [`Device`](crate::Device)
    /// will be updated with the new target level for the blinds.
    pub async fn set_target_level(
        &mut self,
        device: &mut crate::device::Device,
        level: u8,
    ) -> anyhow::Result<()> {
        let inner = device.inner_mut();

        if !has_capability(
            inner.capabilities.can_receive.as_ref(),
            &[crate::device::Capability::BlindsState],
        ) {
            anyhow::bail!("device cannot be change for blind state");
        }

        if level > 100 {
            anyhow::bail!("level must be between 0.0 -> 100.0");
        }

        let mut attributes = HashMap::new();
        attributes.insert("blindsTargetLevel", level);

        let mut body = HashMap::new();
        body.insert("attributes", attributes);

        let body: String = serde_json::to_string(&vec![body])?;

        self.client
            .patch({
                make_url(self.ip_address, &format!("/devices/{}", inner.id))?
            })
            .body(body)
            .send()
            .await
            .map_err(|err| anyhow::anyhow!(err))?;

        inner.attributes.blinds_target_level = Some(level);

        Ok(())
    }

    /// List all scenes that is known for the [`Hub`]. This will return an exhaustive list of
    /// [`Scene`](crate::Scene)s.
    pub async fn scenes(&mut self) -> anyhow::Result<Vec<crate::Scene>> {
        self.client
            .get({
                make_url(self.ip_address, "/scenes")?
            })
            .send()
            .await?
            .json::<Vec<crate::Scene>>()
            .await
            .map_err(|err| anyhow::anyhow!(err))
    }

    /// Get a single [`Scene`](crate::Scene) based on its id.
    pub async fn scene(&mut self, id: &str) -> anyhow::Result<crate::Scene> {
        self.client
            .get({
                make_url(self.ip_address, &format!("/scenes/{}", id))?
            })
            .send()
            .await?
            .json::<crate::Scene>()
            .await
            .map_err(|err| anyhow::anyhow!(err))
    }

    /*/// Trigger a [`Scene`](crate::Scene) now. Will work independent of a scheduled scene or not.
    pub async fn trigger_scene(&mut self, scene: &crate::scene::Scene) -> anyhow::Result<()> {
        let inner = scene.inner();

        self.client
            .call(self.create_request(
                http::Method::POST,
                format!("/scenes/{}/trigger", inner.id).as_str(),
                Some(hyper::Body::empty()),
            )?)
            .await?;

        Ok(())
    }*/

    /*/// Undo scene will revert the changes set by the [`Scene`](crate::Scene).
    pub async fn undo_scene(&mut self, scene: &crate::scene::Scene) -> anyhow::Result<()> {
        let inner = scene.inner();

        self.client
            .call(self.create_request(
                http::Method::POST,
                format!("/scenes/{}/undo", inner.id).as_str(),
                Some(hyper::Body::empty()),
            )?)
            .await?;

        Ok(())
    }*/
}

fn make_url(host: Ipv4Addr, path: &str) -> Result<url::Url, crate::Error> {
    use Part::*;

    url_builder! {
        Scheme("https");
        HostIpv4(host);
        Port(DIRIGERA_PORT);
        Path(&format!("{}{}", DIRIGERA_API_VERSION, path));
    }
    .map_err(crate::Error::UrlBuilder)
}

fn has_capability(
    got: &[crate::device::Capability],
    required: &[crate::device::Capability],
) -> bool {
    required.iter().all(|item| got.contains(item))
}

