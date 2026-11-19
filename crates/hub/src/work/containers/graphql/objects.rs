//! The K1 object types of the containers of a device: each field of what Docker says.

use async_graphql::Object;
use chrono::{DateTime, Utc};

use crate::work::containers::{
    Container, ContainerDetail, ContainerLog, ContainerState, Health, HealthCheck, HealthRun,
    Label, LogLine, Mount, NetworkAddress, Process, ProjectState,
};

#[Object]
impl Container {
    async fn id(&self) -> &str {
        &self.id
    }

    async fn name(&self) -> &str {
        &self.name
    }

    async fn image(&self) -> &str {
        &self.image
    }

    async fn state(&self) -> ContainerState {
        self.state
    }

    async fn status(&self) -> &str {
        &self.status
    }

    async fn health(&self) -> Health {
        self.health
    }

    async fn ports(&self) -> &[String] {
        &self.ports
    }

    async fn created_at(&self) -> Option<DateTime<Utc>> {
        self.created_at
    }

    async fn compose_project(&self) -> Option<&str> {
        self.compose_project.as_deref()
    }

    async fn compose_service(&self) -> Option<&str> {
        self.compose_service.as_deref()
    }

    /// The one state of the container's Compose project (HOST-019); null with no project.
    async fn compose_project_state(&self) -> Option<ProjectState> {
        self.project_state
    }
}

#[Object]
impl ContainerLog {
    async fn lines(&self) -> &[LogLine] {
        &self.lines
    }

    async fn cut(&self) -> bool {
        self.cut
    }
}

#[Object(name = "ContainerLogLine")]
impl LogLine {
    async fn at(&self) -> Option<DateTime<Utc>> {
        self.at
    }

    async fn text(&self) -> &str {
        &self.text
    }
}

#[Object]
impl ContainerDetail {
    async fn id(&self) -> &str {
        &self.id
    }

    async fn name(&self) -> &str {
        &self.name
    }

    async fn image(&self) -> &str {
        &self.image
    }

    async fn image_id(&self) -> &str {
        &self.image_id
    }

    async fn command(&self) -> &[String] {
        &self.command
    }

    async fn working_dir(&self) -> &str {
        &self.working_dir
    }

    async fn user(&self) -> &str {
        &self.user
    }

    async fn restart_policy(&self) -> &str {
        &self.restart_policy
    }

    async fn state(&self) -> ContainerState {
        self.state
    }

    async fn exit_code(&self) -> i32 {
        self.exit_code
    }

    async fn started_at(&self) -> Option<DateTime<Utc>> {
        self.started_at
    }

    async fn finished_at(&self) -> Option<DateTime<Utc>> {
        self.finished_at
    }

    async fn restart_count(&self) -> u32 {
        self.restart_count
    }

    async fn health_check(&self) -> Option<&HealthCheck> {
        self.health_check.as_ref()
    }

    async fn health(&self) -> Health {
        self.health
    }

    async fn failing_streak(&self) -> u32 {
        self.failing_streak
    }

    async fn last_health_run(&self) -> Option<HealthRun> {
        self.last_health_run
    }

    async fn mounts(&self) -> &[Mount] {
        &self.mounts
    }

    async fn networks(&self) -> &[NetworkAddress] {
        &self.networks
    }

    async fn labels(&self) -> &[Label] {
        &self.labels
    }

    /// The names of the environment variables; never a value (HOST-017).
    async fn environment_names(&self) -> &[String] {
        &self.environment_names
    }

    async fn processes(&self) -> &[Process] {
        &self.processes
    }
}

#[Object(name = "ContainerHealthCheck")]
impl HealthCheck {
    async fn test(&self) -> &[String] {
        &self.test
    }

    async fn interval_seconds(&self) -> Option<u32> {
        self.interval_seconds
    }
}

#[Object(name = "ContainerHealthRun")]
impl HealthRun {
    async fn ended_at(&self) -> Option<DateTime<Utc>> {
        self.ended_at
    }

    async fn exit_code(&self) -> i32 {
        self.exit_code
    }
}

#[Object(name = "ContainerMount")]
impl Mount {
    #[graphql(name = "type")]
    async fn kind(&self) -> &str {
        &self.kind
    }

    async fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    async fn source(&self) -> &str {
        &self.source
    }

    async fn destination(&self) -> &str {
        &self.destination
    }

    async fn read_only(&self) -> bool {
        self.read_only
    }
}

#[Object(name = "ContainerNetworkAddress")]
impl NetworkAddress {
    async fn network(&self) -> &str {
        &self.network
    }

    async fn ipv4(&self) -> Option<&str> {
        self.ipv4.as_deref()
    }

    async fn ipv6(&self) -> Option<&str> {
        self.ipv6.as_deref()
    }

    async fn gateway(&self) -> Option<&str> {
        self.gateway.as_deref()
    }

    async fn mac_address(&self) -> Option<&str> {
        self.mac_address.as_deref()
    }
}

#[Object(name = "ContainerLabel")]
impl Label {
    async fn name(&self) -> &str {
        &self.name
    }

    async fn value(&self) -> &str {
        &self.value
    }
}

#[Object(name = "ContainerProcess")]
impl Process {
    async fn pid(&self) -> u32 {
        self.pid
    }

    async fn user(&self) -> Option<&str> {
        self.user.as_deref()
    }

    async fn command(&self) -> &str {
        &self.command
    }
}
