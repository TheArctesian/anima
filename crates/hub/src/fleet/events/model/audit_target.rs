//! What an audit entry is about: one variant for each sort of record that a change names
//! (LOG-005). Its text form is stored in `audit_log.target_type`.

use crate::ids::{
    ApiKeyId, ArchiveId, AttachmentId, BuildJobId, BuilderId, ChannelId, CheckId, DeviceId,
    IngestTokenId, JobId, OperationalKeyId, OperatorId, PlaceId, ProjectGroupId, ProjectId,
    ProposalId, RecordLinkId, ReleaseId, ReminderRuleId, RepositoryId, SessionId, SiteId,
    SourceAccountId, WindowId,
};

/// The thing an audited action changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AuditTarget {
    Operator(OperatorId),
    Site(SiteId),
    Device(DeviceId),
    Check(CheckId),
    Window(WindowId),
    Session(SessionId),
    /// The SSH credential of a device.
    Credential(DeviceId),
    /// The accepted SSH host key of a device.
    HostKey(DeviceId),
    /// A file on a device, moved over the files tunnel; the change holds its path.
    File(DeviceId),
    /// A hub setting, by its dotted name (for example `retention.events`).
    Setting(&'static str),
    /// A notification channel.
    Channel(ChannelId),
    /// A release and its build jobs (BLD-082).
    Release(ReleaseId),
    /// One build job of a release (BLD-082).
    BuildJob(BuildJobId),
    /// A computer enrolled to build releases (BLD-082).
    Builder(BuilderId),
    /// An API key for scripts (D-23).
    ApiKey(ApiKeyId),
    /// The VPN: a key, a registration or a route made through the broker (VP-7). It has no
    /// id of its own; the change names the site or node.
    Vpn,
    /// The device link: an invite or a revoked device (FRN-009). It has no id of its own; the
    /// change names the device, never an invite (LOG-007).
    Link,
    /// A connection to an outside source (SRC-003).
    SourceAccount(SourceAccountId),
    /// A project, its repositories or its resources (PRO-001).
    Project(ProjectId),
    /// A group of projects (PRO-059).
    ProjectGroup(ProjectGroupId),
    /// A project repository on GitHub, for a change the hub sent there; the change names the
    /// issue, pull request, run or commit (PRJ-028, OD-E14).
    Repository(RepositoryId),
    /// A link between two records (RLK-001).
    RecordLink(RecordLinkId),
    /// A file of one record (FIL-001).
    Attachment(AttachmentId),
    /// An archive of one import (FIL-004).
    Archive(ArchiveId),
    /// An extraction proposal (VIS-001).
    ExtractionProposal(ProposalId),
    /// A place, or the site rows that point at it (PLC-001, PLC-006).
    Place(PlaceId),
    /// An operational key, its versions and its consumers (R-147); never its value.
    OperationalKey(OperationalKeyId),
    /// The location samples: a batch that was stored or a span that was read. It has no id
    /// of its own; the change holds counts and times, never a position (K-E4 Section 13).
    LocationSamples,
    /// An ingest token of the OwnTracks path; never its value (K-E4 Section 11).
    IngestToken(IngestTokenId),
    /// A reminder rule (K-E6 Section 2).
    ReminderRule(ReminderRuleId),
    /// A job of the work route (K10 Section 10).
    Job(JobId),
}

impl AuditTarget {
    /// The text stored in `audit_log.target_type`.
    #[must_use]
    pub const fn kind(self) -> &'static str {
        match self {
            Self::Operator(_) => "operator",
            Self::Site(_) => "site",
            Self::Device(_) => "device",
            Self::Check(_) => "check",
            Self::Window(_) => "maintenance_window",
            Self::Session(_) => "session",
            Self::Credential(_) => "credential",
            Self::HostKey(_) => "host_key",
            Self::File(_) => "file",
            Self::Setting(_) => "setting",
            Self::Channel(_) => "channel",
            Self::Release(_) => "release",
            Self::BuildJob(_) => "build_job",
            Self::Builder(_) => "builder",
            Self::ApiKey(_) => "api_key",
            Self::Vpn => "vpn",
            Self::Link => "link",
            Self::SourceAccount(_) => "source_account",
            Self::Project(_) => "project",
            Self::ProjectGroup(_) => "project_group",
            Self::Repository(_) => "repository",
            Self::RecordLink(_) => "record_link",
            Self::Attachment(_) => "attachment",
            Self::Archive(_) => "archive",
            Self::ExtractionProposal(_) => "extraction_proposal",
            Self::Place(_) => "place",
            Self::OperationalKey(_) => "operational_key",
            Self::LocationSamples => "location_samples",
            Self::IngestToken(_) => "location_ingest_token",
            Self::ReminderRule(_) => "reminder_rule",
            Self::Job(_) => "job",
        }
    }

    /// The UUID text stored in `audit_log.target_id`.
    #[must_use]
    pub fn id(self) -> String {
        match self {
            Self::Operator(id) => id.to_string(),
            Self::Site(id) => id.to_string(),
            Self::Device(id) | Self::Credential(id) | Self::HostKey(id) | Self::File(id) => {
                id.to_string()
            }
            Self::Check(id) => id.to_string(),
            Self::Window(id) => id.to_string(),
            Self::Session(id) => id.to_string(),
            Self::Setting(name) => name.to_owned(),
            Self::Channel(id) => id.to_string(),
            Self::Release(id) => id.to_string(),
            Self::BuildJob(id) => id.to_string(),
            Self::Builder(id) => id.to_string(),
            Self::ApiKey(id) => id.to_string(),
            Self::Vpn => "vpn".to_owned(),
            Self::Link => "link".to_owned(),
            Self::SourceAccount(id) => id.to_string(),
            Self::Project(id) => id.to_string(),
            Self::ProjectGroup(id) => id.to_string(),
            Self::Repository(id) => id.to_string(),
            Self::RecordLink(id) => id.to_string(),
            Self::Attachment(id) => id.to_string(),
            Self::Archive(id) => id.to_string(),
            Self::ExtractionProposal(id) => id.to_string(),
            Self::Place(id) => id.to_string(),
            Self::OperationalKey(id) => id.to_string(),
            Self::LocationSamples => "location_samples".to_owned(),
            Self::IngestToken(id) => id.to_string(),
            Self::ReminderRule(id) => id.to_string(),
            Self::Job(id) => id.to_string(),
        }
    }
}
