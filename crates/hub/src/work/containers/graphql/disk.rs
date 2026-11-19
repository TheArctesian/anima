//! The K1 object types of the images, the volumes and the networks of a device (HOST-020).

use async_graphql::Object;

use crate::work::containers::{DiskUsage, Image, Network, Storage, Volume};

#[Object(name = "ContainerStorage")]
impl Storage {
    async fn images(&self) -> &[Image] {
        &self.images
    }

    async fn volumes(&self) -> &[Volume] {
        &self.volumes
    }

    async fn usage(&self) -> &[DiskUsage] {
        &self.usage
    }
}

#[Object(name = "ContainerImage")]
impl Image {
    async fn id(&self) -> &str {
        &self.id
    }

    async fn repository(&self) -> Option<&str> {
        self.repository.as_deref()
    }

    async fn tag(&self) -> Option<&str> {
        self.tag.as_deref()
    }

    async fn size(&self) -> &str {
        &self.size
    }

    async fn shared_size(&self) -> &str {
        &self.shared_size
    }

    async fn unique_size(&self) -> &str {
        &self.unique_size
    }

    async fn created(&self) -> &str {
        &self.created
    }

    async fn container_count(&self) -> u32 {
        self.container_count
    }

    async fn containers(&self) -> &[String] {
        &self.containers
    }
}

#[Object(name = "ContainerVolume")]
impl Volume {
    async fn name(&self) -> &str {
        &self.name
    }

    async fn driver(&self) -> &str {
        &self.driver
    }

    async fn size(&self) -> &str {
        &self.size
    }

    async fn container_count(&self) -> u32 {
        self.container_count
    }

    async fn containers(&self) -> &[String] {
        &self.containers
    }
}

#[Object(name = "ContainerDiskUsage")]
impl DiskUsage {
    #[graphql(name = "type")]
    async fn kind(&self) -> &str {
        &self.kind
    }

    async fn total_count(&self) -> u32 {
        self.total_count
    }

    async fn active_count(&self) -> u32 {
        self.active_count
    }

    async fn size(&self) -> &str {
        &self.size
    }

    async fn reclaimable(&self) -> &str {
        &self.reclaimable
    }
}

#[Object(name = "ContainerNetwork")]
impl Network {
    async fn id(&self) -> &str {
        &self.id
    }

    async fn name(&self) -> &str {
        &self.name
    }

    async fn driver(&self) -> &str {
        &self.driver
    }

    async fn scope(&self) -> &str {
        &self.scope
    }

    async fn internal(&self) -> bool {
        self.internal
    }

    async fn containers(&self) -> &[String] {
        &self.containers
    }
}
