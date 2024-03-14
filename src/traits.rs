#[async_trait::async_trait]
pub trait DirigeraExt {
    type Rejection;
    type Config;
    type Device;

    fn new(config: &Self::Config) -> Result<Self, Self::Rejection>
    where Self: Sized;

    async fn list(&self) -> Result<Vec<Self::Device>, Self::Rejection>
    where Self: Sized;

    async fn get(&self, id: &str) -> Result<Self::Device, Self::Rejection>
    where Self: Sized;
}

