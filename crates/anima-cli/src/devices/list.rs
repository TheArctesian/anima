//! `anima device list`: filters, order and pages (CMD-026).

use super::documents::{DeviceList, SiteNames, device_list, site_names};
use crate::failure::Failure;
use crate::hub::Hub;
use crate::invocation::Invocation;
use crate::output::text::enum_name;
use crate::output::{Cell, Table};

/// The hub's largest page (devices `page.rs`); `--all` asks for pages of this size.
pub const MAX_PAGE: i64 = 200;

#[derive(Debug, clap::Args)]
pub struct ListArgs {
    /// Only devices with this status; repeat for more than one.
    #[arg(long, value_enum)]
    status: Vec<StatusArg>,
    /// Only devices with this tag; repeat for more than one.
    #[arg(long)]
    tag: Vec<String>,
    /// Only devices whose name holds this text (any case).
    #[arg(long)]
    name: Option<String>,
    /// Only devices at this site, by name or ID.
    #[arg(long)]
    site: Option<String>,
    /// The order: name, status (worst first) or uptime (lowest first).
    #[arg(long, value_enum, default_value_t = SortArg::Name)]
    sort: SortArg,
    /// Devices per page, 1 to 200.
    #[arg(long, default_value_t = 50, value_parser = clap::value_parser!(i64).range(1..=MAX_PAGE))]
    limit: i64,
    /// Start after this cursor (from the previous page).
    #[arg(long, conflicts_with = "all")]
    after: Option<String>,
    /// Every page, not only the first.
    #[arg(long)]
    all: bool,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum StatusArg {
    Up,
    Down,
    Degraded,
    Maintenance,
    Unknown,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum SortArg {
    Name,
    Status,
    Uptime,
}

/// One page request: the filter, the order and the page size.
pub struct Query {
    pub filter: device_list::DeviceFilter,
    pub sort: device_list::DeviceSort,
    pub first: i64,
}

impl ListArgs {
    /// Prints the devices as a table or, with `--json`, as the K1 `DeviceConnection`.
    ///
    /// # Errors
    /// [`Failure`] for an unknown site or a hub refusal.
    pub async fn run(self, invocation: &Invocation) -> Result<(), Failure> {
        let hub = invocation.hub()?;
        let site_id = match &self.site {
            Some(site) => Some(site_id(&hub, site).await?),
            None => None,
        };
        let query = Query {
            filter: device_list::DeviceFilter {
                status: Some(self.status.iter().map(|status| status.k1()).collect())
                    .filter(|statuses: &Vec<_>| !statuses.is_empty()),
                tags: Some(self.tag).filter(|tags| !tags.is_empty()),
                name: self.name,
                site_id,
            },
            sort: self.sort.k1(),
            first: if self.all { MAX_PAGE } else { self.limit },
        };
        let devices = fetch(&hub, &query, self.after, self.all).await?;
        invocation.output.data(&devices, table)?;
        if devices.page_info.has_next_page {
            let cursor = devices.page_info.end_cursor.as_deref().unwrap_or_default();
            invocation.output.note(&format!(
                "{} of {} devices. Next page: anima device list --after {cursor} (or --all)",
                devices.edges.len(),
                devices.total_count
            ));
        }
        Ok(())
    }
}

/// The first page after `after`, or with `all` every page joined into one connection.
///
/// # Errors
/// The hub's refusal.
pub async fn fetch(
    hub: &Hub,
    query: &Query,
    after: Option<String>,
    all: bool,
) -> Result<device_list::DeviceListDevices, Failure> {
    let mut joined: Option<device_list::DeviceListDevices> = None;
    let mut after = after;
    loop {
        let variables = device_list::Variables {
            filter: Some(query.filter.clone()),
            sort: Some(query.sort.clone()),
            first: Some(query.first),
            after: after.take(),
        };
        let page = hub.run::<DeviceList>(variables).await?.devices;
        let next = page
            .page_info
            .end_cursor
            .clone()
            .filter(|_| page.page_info.has_next_page);
        let mut devices = match joined.take() {
            Some(mut devices) => {
                devices.edges.extend(page.edges);
                devices.page_info = page.page_info;
                devices.total_count = page.total_count;
                devices
            }
            None => page,
        };
        match next {
            Some(cursor) if all => {
                after = Some(cursor);
                joined = Some(devices);
            }
            _ => {
                devices.page_info.has_next_page &= !all;
                return Ok(devices);
            }
        }
    }
}

/// The ID of the site `site` names: an ID as it is, or the site with exactly that name.
async fn site_id(hub: &Hub, site: &str) -> Result<String, Failure> {
    if uuid::Uuid::parse_str(site).is_ok() {
        return Ok(site.to_owned());
    }
    let sites = hub.run::<SiteNames>(site_names::Variables).await?.sites;
    sites
        .into_iter()
        .find(|candidate| candidate.name == site)
        .map(|found| found.id)
        .ok_or_else(|| Failure::NotFound {
            message: format!("There is no site named {site}."),
            hint: Some("anima site list".to_owned()),
        })
}

fn table(devices: &device_list::DeviceListDevices) -> Table {
    let mut table = Table::new(vec!["NAME", "STATUS", "SITE", "HOST", "TAGS", "ID"]);
    for edge in &devices.edges {
        let device = &edge.node;
        table.row(vec![
            device.name.as_str().into(),
            Cell::status(&enum_name(&device.status)),
            device.site.name.as_str().into(),
            device.host.as_str().into(),
            device.tags.join(",").into(),
            device.id.as_str().into(),
        ]);
    }
    table
}

impl StatusArg {
    const fn k1(self) -> device_list::Status {
        match self {
            Self::Up => device_list::Status::UP,
            Self::Down => device_list::Status::DOWN,
            Self::Degraded => device_list::Status::DEGRADED,
            Self::Maintenance => device_list::Status::MAINTENANCE,
            Self::Unknown => device_list::Status::UNKNOWN,
        }
    }
}

impl SortArg {
    const fn k1(self) -> device_list::DeviceSort {
        match self {
            Self::Name => device_list::DeviceSort::NAME,
            Self::Status => device_list::DeviceSort::STATUS,
            Self::Uptime => device_list::DeviceSort::UPTIME,
        }
    }
}
