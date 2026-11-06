# Anima Design Specification

| Item | Value |
| --- | --- |
| Document | Anima design specification |
| Version | 0.1 (draft) |
| Date | 2026-11-03 |
| Language | ASD-STE100 Simplified Technical English (STE) |
| Status | For review |

## 0. Writing rules for this document

This document uses Simplified Technical English (STE). Obey these rules when you change this document:

1. Write one instruction in one sentence.
2. Keep a procedural sentence to 20 words or fewer.
3. Keep a descriptive sentence to 25 words or fewer.
4. Use the active voice.
5. Use "must" for a requirement. Use "can" for a permitted action.
6. Use one term for one item. Use the terms in Section 2.
7. Keep a paragraph to 6 sentences or fewer.

Each requirement has an identifier, for example `DEV-001`. Do not change an identifier after you publish it.

## 1. Purpose and scope

### 1.1 Purpose

Anima is an application that manages the devices on a home network or a small private network. Anima has three primary functions:

- Remote access. The operator connects to a device with SSH or RDP.
- Monitoring. Anima checks the uptime of each device. Anima also records the resource usage of each device.
- Device management. A later version controls virtual machines and containers on a hypervisor, for example Proxmox VE.

### 1.2 Reference products

Anima uses the functions of these products as a reference:

| Product | Licence | Function that Anima uses as a reference |
| --- | --- | --- |
| Apache Guacamole (`guacamole-server`) | Apache 2.0 | Remote access with SSH, RDP and VNC through a gateway |
| Uptime Kuma | MIT | Uptime checks, status history and notifications |
| Proxmox VE | AGPL 3.0 | Management of virtual machines and containers |

Anima uses `guacd` from Guacamole as a separate process. Anima does not copy source code from Uptime Kuma or Proxmox VE.

### 1.3 Target platforms

| Platform | Package | Phase |
| --- | --- | --- |
| macOS | Tauri desktop application | 1 |
| Linux | Tauri desktop application | 1 |
| Web | Static web application that the hub serves | 3 |
| iOS | Tauri mobile application | 5 |

The desktop applications are the first priority. Do not start the web and iOS work until Phase 2 is complete.

### 1.4 Out of scope for this version

- Windows desktop package. Tauri supports Windows, so a later version can add it.
- Android package.
- Multiple operators with different permissions.
- File transfer between the client and a device.
- Access from outside the private network. Each client must be on the same network as the hub and the devices (Section 4.1).

## 2. Terms

| Term | Meaning |
| --- | --- |
| Operator | The person who uses Anima. |
| Device | A computer, server, virtual machine or network appliance that Anima manages. |
| Client | The Anima application that the operator uses: desktop, web or iOS. |
| Hub | The Anima server process. It runs all the time on the private network. |
| Gateway | The `guacd` process. It converts RDP and VNC into the Guacamole protocol. SSH does not use it: the hub runs SSH itself (R-062). |
| Session | One remote access connection from the client to a device. |
| Check | One test that finds if a device or a service is available. |
| Heartbeat | The result of one check at one time. |
| Metric | One measured value of resource usage, for example CPU usage. |
| Credential | A user name with a password, or a user name with an SSH private key. |

## 3. System architecture

### 3.1 Overview

Anima has three components: the client, the hub and the gateway.

```text
+-------------------------+          +------------------------------------+
| Client                  |  HTTPS   | Hub (Docker host on the network)   |
| - Tauri desktop (macOS, |  + WSS   |                                    |
|   Linux)                +--------->+  anima-hub (Rust, axum)            |
| - Web (Phase 3)         |          |    | GraphQL API (async-graphql)   |
| - iOS (Phase 5)         |          |    | WebSocket tunnel              |
+-------------------------+          |    | Check scheduler               |
                                     |    | Metric collector              |
                                     |    | Logger                        |
                                     |    v                               |
                                     |  guacd (gateway, port 4822)        |
                                     |  PostgreSQL (port 5432)            |
                                     +----+--------------+----------------+
                                          | SSH / RDP    | ICMP, TCP, HTTP,
                                          v              v DNS, SSH, API
                                     +---------------------------+
                                     | Devices                   |
                                     +---------------------------+
```

### 3.2 Reasons for this architecture

1. Uptime monitoring must continue when the client is closed. Thus a process must run all the time. The hub is this process.
2. A web browser cannot open an SSH or RDP connection. Thus the web client needs a server between the browser and the device.
3. The gateway gives one remote access method to all clients. The desktop, web and iOS clients use the same display code (`guacamole-common-js`).
4. The gateway already supports SSH, RDP, VNC and Telnet. Anima does not write a new RDP implementation.

### 3.3 Client

| Item | Decision |
| --- | --- |
| Fonts and icons | Fira Code by default; OpenDyslexic Nerd Font as a Settings choice; Nerd Font icons. See Section 5.2. |
| Framework | Tauri 2 |
| Native layer | Rust |
| User interface | TypeScript in the Tauri web view. See Section 11 for the framework decision. |
| Remote display | `guacamole-common-js` |
| Secret storage | The operating system key store: macOS Keychain, Linux Secret Service |

- **CLI-001.** The client must use the same user interface code on all platforms.
- **CLI-002.** The client must keep the hub address in local settings.
- **CLI-003.** The client must keep the hub access token in the operating system key store.
- **CLI-004.** The client must not keep device credentials. The hub keeps device credentials.

### 3.4 Hub

| Item | Decision |
| --- | --- |
| Language | Rust |
| Web framework | axum |
| API | GraphQL with `async-graphql`. See Section 3.7. |
| Database | PostgreSQL. See Section 3.6. |
| Package | Docker image. A Docker Compose file starts the hub, the gateway and the database together. |

- **HUB-001.** The hub must supply a GraphQL API for devices, checks, metrics, logs and settings.
- **HUB-002.** The hub must supply a WebSocket tunnel between the client and the gateway.
- **HUB-003.** The hub must run the checks on a schedule.
- **HUB-004.** The hub must collect the metrics on a schedule.
- **HUB-005.** The hub must keep heartbeats and metrics for the periods in DB-009. A later setting can change these periods.

### 3.5 Gateway

- **GW-001.** The gateway must use the official `guacamole/guacd` Docker image as its base. The Anima image adds only fonts (RA-011; since R-062 SSH no longer needs the terminal font in the gateway).
- **GW-002.** Only the hub must connect to the gateway. Do not publish port 4822 outside the Docker network.
- **GW-003.** The gateway has no authentication. A process that can connect to port 4822 can open a connection to any host. Thus GW-002 is a security requirement.
- **GW-004.** If the gateway runs on a different host in a later version, the hub must connect to it with TLS.

The gateway does only the protocol work: RDP and VNC connections (SSH goes through the hub's terminal tunnel, R-062), the display, recordings, file streams and joins to a shared session. The Guacamole Java web application does all other work. Anima does not use the Java web application. Thus the hub must do this work:

- **GW-010. Handshake.** The hub must send `select`, read `args` and agree the protocol version. Then the hub must send `size`, `audio`, `video`, `image`, `timezone` and `name`. The hub must send `connect` last, with a value for each argument. Then the hub must read `ready`.
- **GW-011. Tunnel.** The hub must relay the WebSocket subprotocol `guacamole` to the gateway. The hub must answer the internal tunnel instructions and `ping`. The hub must close the tunnel when it gets an instruction that is not valid.
- **GW-012. Parameters.** The hub must make all connection arguments from the database and the credential store. The client can set only these values: screen size, DPI, time zone and terminal colour scheme.
- **GW-013. Forbidden client values.** The client must not set `hostname`, `port`, credentials, `recording-path`, `drive-path`, `sftp-root-directory`, `typescript-path` or `wol-*` values.
- **GW-014. Session control.** The hub must keep the gateway connection ID of each live session. The client must not see this ID.
- **GW-015. Credential prompt.** If the gateway asks for a missing credential with `required`, the client must ask the operator (RDP and VNC; SSH has no prompt for a missing credential since R-062, K4 Section 8.2). The hub must not keep this value unless the operator selects "Save".

### 3.6 Database

Anima uses PostgreSQL. For this version, the database runs on the same host as the hub, in the same Docker Compose file. A later version can move the database to a different host.

| Item | Decision |
| --- | --- |
| Version | PostgreSQL 17 or later |
| Rust access | `sqlx` with compile-time checked queries |
| Schema changes | `sqlx` migrations in the hub repository. A one-shot `migrate` step applies them as the owner role before the hub starts. The hub serves as a runtime role that owns nothing (LOG-006). |
| Data volume | A named Docker volume |

- **DB-001.** Only the hub must connect to the database. Do not publish port 5432 outside the Docker network.
- **DB-002.** The hub must read the database password from a Docker secret or from an environment file. Do not write the password in the Compose file.
- **DB-003.** The hub must use a database role that has access to the Anima database only.
- **DB-004.** The heartbeat, metric and log tables must use time partitions with one partition for each day.
- **DB-005.** The hub must delete old partitions when they are older than the retention period. Do not use row-by-row deletion for this.
- **DB-006.** The hub must make a database connection pool at start. The hub must stop with a clear error if the database is not available after 60 seconds.
- **DB-007.** The Docker Compose file must include a backup procedure that uses `pg_dump`.
- **DB-008.** The hub must keep aggregate tables for heartbeats and metrics: one row for each minute, each hour and each day. The hub must update them when it writes raw data.
- **DB-009.** The hub must keep raw heartbeats and metrics for 30 days. Keep minute aggregates for 7 days, hour aggregates for 90 days and day aggregates for 2 years.
- **DB-010.** Each schema migration must be safe to run on a large database. A long data change must run in parts and must continue after an interruption.
- **DB-011.** The schema must define each default value, for example the number of retries. Do not keep defaults only in the code.

### 3.7 API

The hub supplies one GraphQL API at the path `/graphql`.

| Operation type | Use |
| --- | --- |
| Query | Read devices, checks, heartbeats, metrics, sessions, logs and settings. |
| Mutation | Add, change and delete items. Start a session. Pause a check. |
| Subscription | Send live changes to the client: device status, new heartbeats, new metrics, new log entries. |

- **API-001.** The hub must write the API in Rust with `async-graphql` on axum.
- **API-002.** Subscriptions must use the `graphql-transport-ws` protocol over WebSocket.
- **API-003.** Every operation must require a valid access token, except the login mutation.
- **API-004.** The hub must set a maximum query depth of 10 and a maximum query complexity. The hub must refuse a query that is more than these limits.
- **API-005.** The hub must use a data loader for related items, so that one query does not cause many database queries.
- **API-006.** Lists must use cursor pagination.
- **API-007.** The hub must disable schema introspection and the GraphQL playground in production builds.
- **API-008.** The hub must write the schema to a file (`schema.graphql`) at build time. The client must generate its TypeScript types from this file.
- **API-009.** The remote access stream must not use GraphQL. The hub must send it through the separate WebSocket tunnel (HUB-002). A GraphQL mutation gives the client a single-use token for the tunnel.
- **API-010.** A GraphQL error must not show internal details, for example SQL text or file paths, to the client.

### 3.8 Logs

Anima keeps three types of log.

| Log type | Content | Location | Retention |
| --- | --- | --- | --- |
| Process log | Technical events of the hub process: start, stop, errors, warnings, slow queries, scheduler events | Standard output in JSON format. Docker collects it. | Docker log rotation: 10 files of 10 MB |
| Event log | Events that the operator must see: device status changes, check failures, session start and end, Proxmox tasks, errors | PostgreSQL table `event_log` | 90 days |
| Audit log | Each change that the operator makes: login, logout, add, change and delete of items, power actions | PostgreSQL table `audit_log` | 1 year |

- **LOG-001.** The hub must use the Rust `tracing` crate for all process log entries.
- **LOG-002.** Each process log entry must have a time, a level, a module name and a message. Each entry must also have a request ID when it is part of a request.
- **LOG-003.** The operator must be able to set the process log level with the environment variable `ANIMA_LOG`. The default level is `info`.
- **LOG-004.** Each event log entry and each audit log entry must have a time, a severity, a source and a message. It must also have the related device, if there is one.
- **LOG-005.** The audit log must record who did the action, what changed, the time and the client IP address.
- **LOG-006.** The hub must not change or delete an audit log entry before the end of the retention period.
- **LOG-007.** No log must contain a password, a private key, an access token or the content of a session.
- **LOG-008.** The desktop client must write its own process log to the operating system log folder. The operator must be able to open this folder from the Settings page.
- **LOG-009.** The client must show the event log in a Logs panel on the Devices page. It must also show it in the Logs tab of the device detail view. The operator must be able to filter it by time, severity, device and source.
- **LOG-010.** The client must receive new event log entries through a GraphQL subscription.

### 3.9 Desktop releases and updates

The desktop app updates itself, as Sparkle does on macOS. GitHub Actions builds and publishes each release. The app downloads a new version in the background and asks the operator to restart.

```text
push to main ─► version job ─► build macOS + build Linux (parallel; each signs its updater file)
                                         │
                                         ▼
                 feed job: latest.json lists only the platforms that built ─► publish the release
                                                                                     │
desktop app ── at start and every 4 hours ── GET latest.json ── version differs? ── download, verify signature, install
                                                                                     │
                                                                  banner "Restart to update" ─► relaunch
```

| Item | Value |
| --- | --- |
| Host | Public GitHub Releases of `TheArctesian/anima` (the repository is public, so the app needs no credential) |
| Feed URL | `https://updates.anima.ing/latest.json`. It redirects to the latest GitHub release (DNS-007). |
| Version | `0.1.<workflow run number>`, computed once in the version job. Do not commit version changes. |
| Platforms | macOS universal (`.app.tar.gz` for the updater, `Anima.dmg` for people), Linux x86_64 (`.AppImage`) |
| Updater signature | Tauri updater key pair. The public key is in `tauri.conf.json`. The private key is the GitHub secret `TAURI_SIGNING_PRIVATE_KEY`. |
| macOS signature | Ad-hoc (`APPLE_SIGNING_IDENTITY=-`). The workflow uses Developer ID signing and notarization only when all Apple secrets are present. |
| Rollback | `allowDowngrades: true`. To roll back, mark an older release as "latest". |

- **UPD-001.** The desktop app must check the feed at start and then every 4 hours. Development builds, the web client and the iOS client must not check and must not load the updater plugin.
- **UPD-002.** The app must verify the signature of each update with the public key in the app before it installs the update. If the signature is not correct, the app must not install the update. It must record the failure and continue.
- **UPD-003.** The app must download and install an update without a prompt. The app must not restart until the operator selects "Restart to update", because a restart can stop live sessions.
- **UPD-004.** When an update is installed, the client must show a banner with the text "Anima <version> is installed" and a "Restart to update" button. The banner must have `role="status"`.
- **UPD-005.** After one update is installed, the app must stop checking until it restarts.
- **UPD-006.** The Settings page must show an "Updates" row with the current version, the update state and a "Check for updates" button. A failed check must not show an error dialog; the next check runs at the usual time.
- **UPD-007.** The release workflow must run only on a push to `main` and on manual start. It must not run on pull requests (CQ-010). It must not cancel a release run that is in progress.
- **UPD-008.** The workflow must upload all files before it publishes the release. The release becomes "latest", and thus visible to the app, only after `latest.json` is uploaded.
- **UPD-009.** `latest.json` must list only the platforms whose build passed in this run. Do not list a platform with a file from an older version. (With `allowDowngrades`, the app would install that file again at each check.)
- **UPD-010.** Each release must have versioned updater files, stable-named files for people (`Anima.dmg`, `Anima-x86_64.AppImage`) and a `SHA256SUMS` file.
- **UPD-011.** The macOS build job must verify the bundle with `codesign --verify --deep --strict` and must verify that `lipo -archs` gives `x86_64` and `arm64`.
- **UPD-012.** Only the operator makes the updater key pair and sets the GitHub secrets. Agents must not read the private key file. The operator must keep a backup of the private key: if it is lost, each installed app must be installed again by hand.

## 4. Security

The hub can open a session to every device. Thus the hub is a high-value target. Obey all the requirements in this section.

- **SEC-001.** The hub must use TLS for all client connections.
- **SEC-002.** The hub must ask for authentication before it accepts an API request or a tunnel request.
- **SEC-003.** The hub must encrypt credentials before it writes them to the database.
- **SEC-004.** The hub must read the encryption key from a file or a Docker secret. Do not keep the key in the database.
- **SEC-005.** The hub must not send a stored password or private key to the client.
- **SEC-006.** For SSH, the hub must record the host key of the device at the first connection.
- **SEC-007.** If the host key changes, the hub must stop the connection and show a warning to the operator.
- **SEC-008.** The hub must not be available on the public internet. See Section 4.1.
- **SEC-009.** The client must not write a credential or a token to a log file.

### 4.1 Network model

Each computer that runs an Anima client must be on the same network as the hub and the devices. Anima does not supply remote access from the public internet.

The devices are at more than one site. Example: office 1, home and office 2. A mesh VPN joins the sites into one private network. A computer on this VPN is "on the network".

- **NET-001.** The hub must have a list of permitted client networks in CIDR format. The default is the VPN address range and the subnets of the sites.
- **NET-002.** The hub must refuse each API request, tunnel request and login from an address that is not in a permitted network. The hub must write the refusal to the event log.
- **NET-003.** The hub must listen only on its VPN interface and its private network interfaces. Do not bind the hub to a public interface.
- **NET-004.** If the client cannot connect to the hub, the navigation bar must show "Not on the network" and the hub address. The message must tell the operator to check the VPN client. Data on the screen must show its age.
- **NET-005.** The client must find the hub by its DNS name `hub.anima.ing` (DNS-001). mDNS does not cross sites, so use mDNS only as a fallback on the local network. The operator can also type the hub address.
- **NET-006.** A later version can let the hub find new devices on each site with mDNS and an ARP scan. The operator must accept each found device before Anima adds it.

### 4.2 Sites and VPN

```text
                  Hostinger VPS
        +----------------------------------+
        | Headscale (VPN coordination)     |
        | DERP relay (only when necessary) |
        | Anima hub, gateway, PostgreSQL   |  <- on the VPN address only
        +----------------+-----------------+
                         | encrypted, mostly direct between sites
      +------------------+------------------+
  Office 1             Home               Office 2
  Mac (client)         Linux PC           Linux PC
                       (subnet router)    (subnet router)
                       Pi-hole            Mac (client)
```

How it operates:

1. Each machine on the VPN gets a key and a VPN address from Headscale.
2. Headscale tells each machine where the other machines are. It does not carry their traffic.
3. The machines connect directly to each other with WireGuard encryption. If a firewall blocks a direct connection, traffic goes through the DERP relay on the VPS. The relay cannot read the traffic.
4. Anima does not know that the VPN exists. The operating system sends traffic for VPN addresses and site subnets through the VPN. All other traffic uses the normal internet connection.

| Item | Decision |
| --- | --- |
| VPN | Tailscale clients with a self-hosted Headscale control server |
| Control server location | Hostinger VPS |
| Site access | One subnet router at each site that has devices. The router advertises the site subnet. |
| Clients | Each Mac or PC that runs Anima runs the Tailscale client. The client starts at login and stays connected. |
| Appliances | Devices such as the Pi-hole do not need VPN software. They are available through the subnet router. |
| Exit node | Not used. The VPN carries only traffic for VPN addresses and site subnets (split tunnel). |

- **VPN-001.** Each site must use a different subnet, for example home `10.10.0.0/24`, office 1 `10.20.0.0/24`, office 2 `10.30.0.0/24`. Overlapping subnets cause route conflicts.
- **VPN-002.** Each device in Anima must record its site. The device list must have a site filter.
- **VPN-003.** Node keys of subnet routers and of the hub must not expire. Node keys of client computers must expire after 180 days.
- **VPN-004.** The VPN access rules (ACLs) must permit only these connections:
  - Anima client computers to the hub, and to all devices.
  - The hub to all devices, for checks, metrics and sessions.
  - Subnet routers to their own site subnet.
  - Devices and appliances cannot start a connection to client computers.
- **VPN-005.** The hub must show for each site if the path is direct or relayed. A relayed path uses the bandwidth of the VPS.
- **VPN-006.** Wake-on-LAN does not cross sites. A later site agent must send the Wake-on-LAN packet inside the site of the device (RA-031).
- **VPN-007.** The operator can use a different VPN at the same time. A full-tunnel VPN (for example a company VPN) can block the Anima VPN. The setup guide must tell the operator about this.

VPS security:

- **VPS-001.** SSH to the VPS must use keys only. Disable password login and root login.
- **VPS-002.** The VPS firewall must permit only 443/tcp (Headscale and DERP), 3478/udp (STUN) and 41641/udp (WireGuard). All other ports are closed to the internet.
- **VPS-003.** The Headscale administration must be available only on the VPN.
- **VPS-004.** The VPS must install security updates automatically.
- **VPS-005.** The operator must confirm each change to the VPS or the Hostinger account. Automation must not change the VPS without this confirmation.
- **VPS-006.** Back up the Headscale database and the Anima database each day to a site that is not the VPS.

#### 4.2.1 Domain names

The operator owns the domain `anima.ing`. Cloudflare holds its DNS.

| Name | Public DNS record | Points to | Purpose |
| --- | --- | --- | --- |
| `hs.anima.ing` | A, not proxied | VPS public IP | Headscale control server and DERP relay (Headscale `server_url`) |
| `hub.anima.ing` | A, not proxied | Hub VPN address (`100.64.0.0/10`) | Hub address for clients. The name is public, but only VPN members can connect. |
| `updates.anima.ing` | Proxied | Cloudflare redirect rule | Update feed and downloads (Section 3.9) |
| `<device>.net.anima.ing` | None | Headscale DNS only | VPN names of hosts (Headscale `dns.base_domain` = `net.anima.ing`) |

- **DNS-001.** The client must use `hub.anima.ing` as the default hub address (NET-005).
- **DNS-002.** The hub must use a Let's Encrypt certificate for `hub.anima.ing`. Get it with the DNS-01 challenge through the Cloudflare API, because the hub has no public port. Renew it automatically. This satisfies SEC-001 with a certificate that the client trusts without a pinned exception.
- **DNS-003.** The Cloudflare API token for DNS-01 must have only the permission "Zone DNS Edit" for `anima.ing`. Keep it in a Docker secret (SEC-004 rules apply). Do not log it.
- **DNS-004.** The `hs.anima.ing` and `hub.anima.ing` records must not be proxied. Cloudflare cannot proxy WireGuard, STUN or a VPN address.
- **DNS-005.** `hub.anima.ing` points to a private address. A connection from outside the VPN must fail. This keeps SEC-008: the public record shows the name and a VPN address only.
- **DNS-006.** Headscale `dns.base_domain` must be `net.anima.ing`. It must not be `anima.ing`, because Headscale does not permit a base domain that contains the `server_url` host. Device names under `net.anima.ing` do not have public records.
- **DNS-007.** `updates.anima.ing/latest.json` must redirect (HTTP 302) to `https://github.com/TheArctesian/anima/releases/latest/download/latest.json`. `updates.anima.ing/<file>` must redirect to the same file of the latest release. The app keeps the `updates.anima.ing` URL, so the host can change later without a new app build.
- **DNS-008.** The operator must confirm each change to the Cloudflare account or the `anima.ing` zone, as for the VPS (VPS-005).

### 4.3 Traffic data (later)

The VPN encrypts traffic between machines, and most traffic does not go through the VPS. Thus the VPN does not give a central record of traffic. These two sources give traffic data:

| Source | Data | Method |
| --- | --- | --- |
| Pi-hole | DNS queries: time, device, domain, result | Pi-hole API, read by the hub |
| Subnet routers | Flow records: time, source, destination, port, protocol, bytes | `conntrack` or nftables flow logs, sent to the hub |

- **TRF-001.** Anima must collect metadata only. Anima must not record the content of traffic.
- **TRF-002.** Traffic data is sensitive. The client must show it only to the operator. Do not send it to a service outside the network.
- **TRF-003.** At a shared site (for example an office), traffic data can include the activity of other people. The operator must set traffic data on for each site, and must have permission to record it.
- **TRF-004.** The hub must keep raw traffic data for 30 days and summaries for 1 year. A later "memory" function can use these summaries.

## 5. User interface

### 5.1 Colour scheme

Anima uses the Nord colour scheme. The default theme is dark.

| Token | Nord name | Value | Use |
| --- | --- | --- | --- |
| `--bg` | nord0 | `#2E3440` | Main background |
| `--surface` | nord1 | `#3B4252` | Navigation bar, cards, panels |
| `--surface-raised` | nord2 | `#434C5E` | Hover state, selected row, input fields |
| `--border` | nord3 | `#4C566A` | Borders and dividers |
| `--text` | nord6 | `#ECEFF4` | Primary text |
| `--text-muted` | nord4 | `#D8DEE9` | Secondary text and labels |
| `--accent` | nord8 | `#88C0D0` | Primary buttons, links, active navigation item, focus ring |
| `--accent-alt` | nord9 | `#81A1C1` | Secondary buttons and icons |
| `--accent-deep` | nord10 | `#5E81AC` | Pressed state of a primary button |
| `--info` | nord7 | `#8FBCBB` | Information messages |
| `--status-up` | nord14 | `#A3BE8C` | Device or check is up |
| `--status-down` | nord11 | `#BF616A` | Device or check is down. Error messages. |
| `--status-degraded` | nord13 | `#EBCB8B` | Slow response or high resource usage |
| `--status-warning` | nord12 | `#D08770` | Warnings. Host key changed. |
| `--status-maintenance` | nord15 | `#B48EAD` | Device is in maintenance |
| `--status-unknown` | nord3 | `#4C566A` | No data, or the check is paused |

- **UI-001.** All colours in the user interface must come from the tokens in this table.
- **UI-002.** Do not use nord3 for text on nord0 or nord1. The contrast is too low. Use nord4 for secondary text.
- **UI-003.** A status must not use colour as the only indicator. Show a text label or an icon with the colour.
- **UI-004.** Text on an `--accent` background must use nord0.
- **UI-005.** A later version can add a light theme with the Nord Snow Storm colours (nord4 to nord6). Define the tokens so that this change is easy.

### 5.2 Typography and spacing

Anima uses Fira Code for text by default and Nerd Fonts for icons. The operator can change the interface text and the monospace text to OpenDyslexic in Settings, each on its own (UI-020; operator decision, 2026-11-05). The Nerd Fonts project supplies OpenDyslexic with the icons already added (patched), so icons always come from the OpenDyslexic Propo file, whichever text font is chosen.

| Use | Font family | File | Licence |
| --- | --- | --- | --- |
| Interface text, IP addresses, host names, code (default) | Fira Code | `woff2/FiraCode-{Regular,Bold}.woff2` from the `tonsky/FiraCode` 6.2 release | SIL OFL 1.1 |
| Interface text (choice) and icons | OpenDyslexic Nerd Font Propo | `ryanoasis/nerd-fonts`, `patched-fonts/OpenDyslexic/Regular/OpenDyslexicNerdFontPropo-{Regular,Bold,Italic,BoldItalic}.otf` | SIL OFL 1.1 |
| Terminal; IP addresses, host names, code (choice) | OpenDyslexicM Nerd Font Mono | `ryanoasis/nerd-fonts`, `patched-fonts/OpenDyslexic/Mono-Regular/OpenDyslexicMNerdFontMono-Regular.otf` | SIL OFL 1.1 |
| Icon fallback | Symbols Nerd Font | `patched-fonts/NerdFontsSymbolsOnly/SymbolsNerdFont-Regular.ttf` | MIT |

- **UI-010.** Use Fira Code for all interface text by default. OpenDyslexic Nerd Font Propo is the alternative (UI-020).
- **UI-011.** Use Fira Code for IP addresses, host names and code by default; OpenDyslexicM Nerd Font Mono is the alternative (UI-020). The SSH terminal uses OpenDyslexicM Nerd Font Mono (RA-011). Use the "Mono" variant in the terminal, because its icons are one cell wide.
- **UI-014.** Use Nerd Font glyphs for all icons. Do not use image files or a second icon library. Use the Propo variant for icons in the interface, because its icons have their natural width.
- **UI-015.** Each icon must have `aria-hidden="true"`. A button that has only an icon must have a text label for screen readers and a tooltip.
- **UI-016.** Refer to each icon by its Nerd Font class name in the code, for example `nf-md-server`. Keep a table of the class names and code points in one source file. Do not write raw code points in components.
- **UI-017.** The client must include the font files in the application package. Do not load fonts from the internet at run time.
- **UI-022.** The Bold and Italic files of OpenDyslexic Nerd Font Propo are not in the Nerd Fonts v3.4.0 release. They are on the main branch. Pin the font files to commit `f034eeafc0ccf79afa8dd952feb86b18e5d0469e` until a release includes them.
- **UI-018.** Each font licence file must be in the application package and in the "About" information.
- **UI-019.** OpenDyslexic is wider and taller than most system fonts, and Fira Code is monospaced and wide. Use `text-md` (0.875 rem) for body text, `text-sm` (0.8125 rem) for dense tables and `leading-normal` (1.5). Test each layout at these sizes in both typefaces. See Section 5.5.
- **UI-021.** Apple recommends the system font first. Anima uses Fira Code by default and offers OpenDyslexic for readability. This is a deliberate exception. The system font stays the fallback in every font token.
- **UI-020.** The Settings page has two choices, "Interface font" and "Monospace font". Each is Fira Code (the default) or OpenDyslexic, and each is saved on the device. The choice changes the `font-ui` and `font-mono` tokens, so no component reads it. A choice applies at once and before the first render after a restart. Icons do not change. More typefaces or text roles can be added to the same choice later.
- **UI-012.** Use a 4-pixel spacing grid. Use the `space-*` tokens (Section 5.5.3).
- **UI-013.** Interactive controls must have a minimum target size of 24 by 24 pixels on desktop and 44 by 44 points on iOS.

### 5.3 Navigation bar

The navigation bar is on the left side of the window on desktop and web. It is at the bottom of the screen on iOS.

| Position | Item | Icon | Page |
| --- | --- | --- | --- |
| Top | Devices | Server | Devices page (Section 6) |
| Bottom | Settings | Gear | Settings page (Section 7) |

- **NAV-001.** The navigation bar must show the page items at the top. Settings must be the last item, at the bottom of the navigation bar, below a divider.
- **NAV-002.** The navigation bar must show the active item with the `--accent` colour.
- **NAV-003.** The navigation bar must show the connection status of the hub immediately above the Settings item: "Connected" or "Disconnected".
- **NAV-004.** The Devices page must be the start page.
- **NAV-005.** The operator must be able to go to each page with the keyboard.

### 5.4 Design principles

Anima uses the eight design principles from Apple (*Principles of Great Design*) and the fluid interface rules from *Designing Fluid Interfaces* (WWDC 2018). This table shows what each principle means for Anima.

| Principle | Meaning for Anima |
| --- | --- |
| Purpose | Each screen helps the operator find a problem or get into a device. Do not add a function that does neither. |
| Agency | The operator stays in control. Give undo for small actions. Ask for confirmation only for actions that are destructive and cannot be undone, for example delete or stop. |
| Responsibility | Never show a credential. Show the TLS and host key status before the operator trusts a device. |
| Familiarity | Things that look the same must operate the same. A session tab always operates the same, for SSH, RDP, VNC or Web. |
| Flexibility | Desktop has precise pointer control and dense layouts. iOS has touch and short tasks. Adapt the layout to the platform. |
| Simplicity | Show the common path first: status and "Connect". Put advanced settings one level deeper. |
| Craft | Each value for colour, space, time and motion comes from a token (Section 5.5). Do not use a value that you cannot explain. |
| Delight | The result of the other seven principles. Anima must feel calm and confident. Do not add decoration. |

Feedback has four types: status, completion, warning and error. Each screen must answer these questions: Where am I? Where can I go? How do I go back?

### 5.5 Design tokens

A design token is a named design value, for example a colour, a space or a spring. All interface code must use tokens. This makes the design consistent and lets one change update all platforms.

#### 5.5.1 Source and outputs

| Item | Decision |
| --- | --- |
| Format | W3C Design Tokens Community Group format (`.tokens.json`) |
| Location | `design/tokens/` in the repository |
| Build tool | Style Dictionary (version 4 or later) |
| Output: interface | `apps/client/src/styles/tokens.css` (CSS custom properties on `:root`) |
| Output: animation code | `apps/client/src/lib/motion/tokens.ts` (typed constants for springs and thresholds) |
| Output: hub | `crates/theme` (the Nord colour table, and the gateway's terminal colour scheme for the Guacamole tunnel; the SSH terminal's theme comes from `tokens.css`, RA-010) |
| Output: documentation | A generated token page in the design documentation |

- **TOK-001.** The token files are the single source of truth. Do not edit the generated files.
- **TOK-002.** The build must make all outputs from the token files. The gate script (CQ-010) must fail if a generated file is not current.
- **TOK-003.** A CSS gate (Stylelint) must refuse raw values in component code. This applies to colours, durations, easing curves, z-index values and font families. Existing exceptions go in a baseline that can only become smaller.
- **TOK-004.** A token can change its value. A token must not change its meaning. To change a meaning, add a new token and remove the old token.

#### 5.5.2 Token levels

| Level | Purpose | Example name | Example value |
| --- | --- | --- | --- |
| Primitive | A raw value with no meaning | `--nord-8` | `#88C0D0` |
| Semantic | A value with a meaning in the interface | `--accent` | `var(--nord-8)` |
| Component | A value for one component | `--btn-primary-bg` | `var(--accent)` |

- **TOK-010.** Component code must use semantic tokens or component tokens. Component code must not use primitive tokens.
- **TOK-011.** A theme (dark, light, high contrast) changes only semantic tokens. The light theme (UI-005) and a high-contrast theme must not need changes to components.

#### 5.5.3 Token groups

| Group | Tokens | Values |
| --- | --- | --- |
| Colour, primitive | `nord-0` to `nord-15` | Section 5.1 |
| Colour, semantic | `bg`, `surface`, `surface-raised`, `border`, `text`, `text-muted`, `accent`, `accent-alt`, `accent-deep`, `info`, `status-*` | Section 5.1 |
| Colour, tint | `accent-tint`, `status-*-tint` | The colour at 12 % on `surface` |
| Font family | `font-ui`, `font-mono`, `font-ui-opendyslexic`, `font-mono-opendyslexic`, `font-icon` | Section 5.2 |
| Font size | `text-xs`, `text-sm`, `text-md`, `text-lg`, `text-xl`, `text-2xl` | 0.75, 0.8125, 0.875, 1, 1.25, 1.5 rem |
| Line height | `leading-tight`, `leading-normal`, `leading-relaxed` | 1.2, 1.5, 1.7 |
| Tracking | `tracking-display`, `tracking-body`, `tracking-small` | −0.01 em, 0, +0.01 em |
| Weight | `weight-regular`, `weight-bold` | 400, 700 (OpenDyslexic has only these two weights; Fira Code ships these two) |
| Space | `space-0` to `space-16` | 0, 0.25, 0.5, 0.75, 1, 1.25, 1.5, 2, 2.5, 3, 4 rem (4-pixel grid) |
| Radius | `radius-sm`, `radius-md`, `radius-lg`, `radius-pill` | 4, 6, 10, 999 px |
| Border width | `border-thin`, `border-focus` | 1, 2 px |
| Icon size | `icon-sm`, `icon-md`, `icon-lg` | 1, 1.125, 1.25 em |
| Target size | `target-min-desktop`, `target-min-touch` | 24 px, 44 pt |
| Elevation | `elevation-0` to `elevation-3` | Section 5.6 |
| Material | `material-chrome`, `material-panel`, `material-overlay` | Section 5.6 |
| Layer (z-index) | `layer-base`, `layer-sticky`, `layer-nav`, `layer-popover`, `layer-modal`, `layer-toast` | 0, 10, 20, 30, 40, 50 |
| Breakpoint | `bp-compact`, `bp-medium`, `bp-wide` | below 768 px, 768 to 1100 px, above 1100 px |
| Motion | `duration-*`, `ease-*`, `spring-*`, `stagger-*`, `threshold-*` | Section 5.7 |

- **TOK-020.** Font sizes and spaces must use `rem`, so that the layout grows with the text size setting of the operating system. This replaces the pixel values in UI-019.
- **TOK-021.** Tracking changes with the font size. Do not use one letter-spacing value for all sizes. OpenDyslexic letters have a heavy base and are wide. Thus use only a small negative tracking for large text.

### 5.6 Materials and depth

The navigation bar, the session toolbar and the logs panel header are floating layers. The content moves below them.

| Token | Use | Value | Reduced transparency |
| --- | --- | --- | --- |
| `material-chrome` | Navigation bar, session toolbar | `surface` at 82 % opacity, blur 20 px, saturation 140 % | `surface`, no blur |
| `material-panel` | Logs panel, side panels | `surface` at 90 % opacity, blur 12 px | `surface`, no blur |
| `material-overlay` | Menus, popovers, toasts | `surface-raised` at 92 % opacity, blur 24 px | `surface-raised`, no blur |
| `elevation-1` | Cards | 1 px border, no shadow | Same |
| `elevation-2` | Popovers, menus | Shadow 0 4 px 16 px, `nord-0` at 40 % | Same |
| `elevation-3` | Modal dialogs, sheets | Shadow 0 12 px 40 px, `nord-0` at 55 % | Same |

- **MAT-001.** A larger surface must have a stronger blur and a deeper shadow than a small surface.
- **MAT-002.** Do not put a translucent surface on a different translucent surface.
- **MAT-003.** A modal task (for example the "Add device" form or a confirmation) must dim the content behind it with a scrim. A panel that does not stop the work (for example the logs panel) must not use a scrim.
- **MAT-004.** Where content moves below floating chrome, use a short fade mask at the edge. Do not use a hard 1-pixel line.
- **MAT-005.** Text on a translucent material must use `text`, not `text-muted`.
- **MAT-006.** The client must obey `prefers-reduced-transparency` and `prefers-contrast: more`. With `prefers-contrast: more`, materials are solid and borders use `text-muted`.
- **MAT-007.** WebKitGTK on Linux can be slow with `backdrop-filter`. Measure the frame time of the Devices page on the reference Linux computer. If it is more than 16.7 ms, the Linux build must use the "reduced transparency" values.
- **MAT-008.** Never animate the content of a remote display (SSH terminal, RDP, VNC or Web view). Do not put a material or a blur over a live remote display.

### 5.7 Motion

#### 5.7.1 Rules

Motion in Anima tells the operator something. Examples: "this came from there", "this finished" and "this is the same item". If a motion does not tell the operator something, do not add it.

- **MOT-001. Response.** Show feedback on pointer-down, not on click. A control must change in less than 100 ms after input.
- **MOT-002. Interruption.** The operator must be able to interrupt each motion. Do not block input during a transition. A new motion must start from the current position on the screen, not from the end value of the old motion.
- **MOT-003. Springs.** Use springs for each motion that the operator can touch or interrupt. Use a duration and a curve only for opacity and colour changes.
- **MOT-004. No bounce without momentum.** Use critically damped springs (damping ratio 1.0) by default. Use a bounce (damping ratio 0.8) only after a drag or a flick.
- **MOT-005. Spatial consistency.** An item must leave on the same path that it came in on. A menu, popover or session tab must grow from the control that opened it (`transform-origin` at the control).
- **MOT-006. Performance.** Animate only `transform` and `opacity`. Do not animate `width`, `height`, `top` or `left`. Use the individual `translate`, `scale` and `rotate` properties, so that two motions on one item do not overwrite each other.
- **MOT-007. Layout shift.** Loading must not move content. A placeholder (skeleton) must have the same size as the content that replaces it. The cumulative layout shift of each page must be 0.
- **MOT-008. Readings.** A metric value (CPU, memory, VRAM, response time, uptime) must change with a cross-fade. It must not count up or down, because a count shows values that did not occur. A bar can move to its new length.
- **MOT-009. Honest progress.** A progress indicator must show the real progress. Do not use easing on progress.
- **MOT-010. Do not animate.** Do not animate focus rings, terminal text, remote displays, error text, or values in the audit log. Do not use confetti, particles, glows or bounces as decoration.
- **MOT-011. Ambient motion.** The only motion that repeats is the "Live" indicator for GraphQL subscriptions and the terminal cursor. Each one must stop when the window is not visible and with reduced motion.

#### 5.7.2 Motion tokens

Springs use the two Apple parameters: damping ratio and response. The token build converts them to the values of the spring library with these formulas:

- stiffness = (2π ÷ response)²
- damping = 4π × damping ratio ÷ response
- mass = 1

| Token | Value | Use |
| --- | --- | --- |
| `spring-default` | damping 1.0, response 0.35 s | Most motion |
| `spring-snappy` | damping 1.0, response 0.25 s | Menus, popovers, tab selection indicator |
| `spring-move` | damping 1.0, response 0.4 s | Reorder, shared items (card to detail) |
| `spring-momentum` | damping 0.8, response 0.3 s | After a drag or a flick: session tabs, toasts, sheets, panel resize |
| `duration-instant` | 0 ms | State changes that must not move |
| `duration-press` | 100 ms | Press feedback |
| `duration-fast` | 150 ms | Exits, hover colour |
| `duration-base` | 200 ms | Cross-fades, theme change |
| `duration-highlight` | 1500 ms | Fade of the highlight on a new log row or a status change |
| `ease-enter` | `cubic-bezier(0.2, 0, 0, 1)` | Opacity and colour in |
| `ease-exit` | `cubic-bezier(1, 0, 0.8, 1)` | Opacity and colour out (mirror of `ease-enter`) |
| `stagger-list` | 20 ms, maximum 8 items | List items that come in together |
| `threshold-drag` | 10 px | Movement before a drag starts |
| `threshold-loading` | 300 ms | Wait before a skeleton or spinner shows |
| `threshold-loading-min` | 500 ms | Minimum display time of a spinner after it shows |
| `projection-deceleration` | 0.998 | Momentum projection (MOT-032) |
| `rubberband-constant` | 0.55 | Resistance at a limit (MOT-033) |

- **MOT-020.** The client must use the `motion` library (vanilla JavaScript API) for springs. It works with each user interface framework in OD-1.
- **MOT-021.** Thresholds are not motion. Reduced motion must not change `threshold-*` tokens.

#### 5.7.3 Motion roles

| Role | Where | Enter | Exit | Reduced motion |
| --- | --- | --- | --- | --- |
| Press | Buttons, cards, tabs | Scale 0.97 on pointer-down, `duration-press` | `spring-default` to 1 on release | Colour change only |
| Hover | Controls, rows | Colour, `duration-fast` | Same | Same |
| Disclosure | Logs panel, check settings, form sections | Height by FLIP with `spring-default`, content fade | Same path in reverse | Instant height, fade `duration-base` |
| Menu and popover | Sort, tags, Web menu, Keys menu | Scale 0.96 to 1 and fade, from the control, `spring-snappy` | Fade and scale to 0.98, `duration-fast` | Fade only |
| Modal | Add device, confirmations, host key warning | Scrim fades in. Surface materializes: blur and scale 0.98 to 1, `spring-default` | Same path in reverse, `duration-fast` | Fade only |
| Page change | Devices and Settings | Cross-fade `duration-base` | Same | Same |
| Card to detail | Device card to device detail | The device name and status move from the card to the header (shared item), `spring-move` | The header moves back to the card | Cross-fade |
| Session tab open | SSH, RDP, VNC, Web buttons | The tab grows from the button that opened it, `spring-snappy` | The tab closes to its place in the tab strip | Fade |
| Reorder | Sort, filter, live status order | Items move to new places by FLIP, `spring-move`. New items fade and scale from 0.98, `stagger-list` | Items fade, `duration-fast`, and keep their place until the fade ends | Instant order, fade |
| Status change | Device card, table row, status pill | Pill cross-fade `duration-base`. Card edge highlight fades over `duration-highlight` | — | Cross-fade only |
| New heartbeat | Heartbeat bar | Bar moves one cell to the left, `spring-default` | — | Instant |
| New log row | Logs panel | Row comes in at the top, other rows move down (FLIP), highlight fades over `duration-highlight` | — | Instant, highlight remains |
| Chart update | Metric charts | New point comes in from the right. The line moves left continuously | Range change: cross-fade | Instant |
| Loading | Lists, charts, detail tabs | After `threshold-loading`: skeleton with the same geometry. Data replaces it with a cross-fade | — | Same, no shimmer |
| Toast | Results of actions | In from the bottom-right, `spring-default` | Out on the same path | Fade |
| Connection steps | Session start | Each step mark changes when the step ends. No easing on progress | — | Same |
| Theme change | Settings | Cross-fade of colours `duration-base` | — | Same |

#### 5.7.4 Gestures (direct manipulation)

| Gesture | Platform | Behaviour |
| --- | --- | --- |
| Resize the logs panel | Desktop | The panel edge follows the pointer 1:1. On release, the panel goes to the snap point nearest the projected position: collapsed, 33 % or 66 %. |
| Resize the navigation bar | Desktop | Follows the pointer 1:1, with rubber-band resistance below the icon-only width and above 280 px. |
| Reorder session tabs | Desktop | The tab follows the pointer from the grab point. Other tabs move away with `spring-move`. |
| Drag a session tab out of the strip | Desktop | Pops the session out to a new window (RA-002). |
| Dismiss a toast | Desktop and iOS | Swipe right. A fast flick dismisses it even when the distance is short. |
| Sheets | iOS | Sheets follow the finger and use momentum projection to open or close. |
| Swipe actions on a device row | iOS | Swipe left for "Connect" and "Edit". Rubber-band at the end of the actions. |

- **MOT-030.** A drag must keep the offset between the pointer and the item at the grab point. Use Pointer Events with `setPointerCapture`.
- **MOT-031.** On release, the spring must start with the velocity of the pointer.
- **MOT-032.** Choose the snap point from the projected position: position + (velocity ÷ 1000) × d ÷ (1 − d), where d is `projection-deceleration`. Do not choose it from the release position.
- **MOT-033.** At a limit, use rubber-band resistance: (overshoot × size × c) ÷ (size + c × |overshoot|), where c is `rubberband-constant`. Do not stop hard.
- **MOT-034.** Decide between "open" and "close" from the direction of the velocity at release, not only from the position.
- **MOT-035.** Use separate springs for the X axis and the Y axis.

#### 5.7.5 Reduced motion

- **MOT-040.** The client must obey `prefers-reduced-motion: reduce`. Each role in Section 5.7.3 has a reduced-motion form. Reduced motion does not mean no feedback. It means a cross-fade or an instant change in place of movement.
- **MOT-041.** The reduced-motion form must also apply to JavaScript springs and to view transitions. A CSS rule on `*` does not reach them. Read the setting in `apps/client/src/lib/motion/` and give it to each spring.
- **MOT-042.** Nothing must wait for a timer that is only there for an animation.
- **MOT-043.** A later Settings item can let the operator set reduced motion in Anima, separately from the operating system.

#### 5.7.6 Multimodal feedback (iOS)

- **MOT-050.** Use haptic feedback only for: a session that connects, a power action that completes, an error, and a snap at the end of a drag.
- **MOT-051.** The haptic feedback and the visual change must occur in the same frame.
- **MOT-052.** The desktop clients do not play sounds.

### 5.8 Motion verification

At the end of each phase, do a motion audit with the `motion-audit` procedure on synthetic data. Do not use real device data.

- **MV-001.** Measure in WebKit (Playwright WebKit), because Tauri uses WebKit on macOS (WKWebView) and Linux (WebKitGTK). Also measure in Chromium for the web client.
- **MV-002.** Classify each finding with the motion-audit classes: dead swap, blinking exit, layout jump, broken continuity, loader without a handoff, missing feedback, missing direct manipulation, numbers that jump, wrong motion, cross-platform drift and performance.
- **MV-003.** Interrupt each motion during the test: a second click during a motion, Escape during an open, Back during a transition, and two dismissals 60 ms apart.
- **MV-004.** Measure layout shift after each interaction with `getBoundingClientRect()` on each frame. The `layout-shift` observer does not report shifts within 500 ms of input.
- **MV-005.** Delay the API by 400 ms in the test, so that staged loading shows.
- **MV-006.** Check that WebKit supports each view transition that Anima uses. If it does not, use a FLIP animation from the `motion` library.
- **MV-007.** Write the report to `docs/research/motion/` with a ranked list. A reel that shows the top proposals is optional.
- **MV-008.** Gates (CQ-010): transitions must use tokens or the shared motion helpers. Each `@keyframes` rule must be in an allowlist with a reason.

## 6. Devices page

### 6.1 Device list

The Devices page shows all devices in a grid of cards. The operator can change the grid to a table.

Each device card shows:

- The device name.
- The host name or the IP address.
- The status: Up, Down, Degraded, Maintenance or Unknown.
- The uptime percentage for the last 24 hours.
- A bar for the last 50 heartbeats.
- CPU usage, memory usage and disk usage, if metrics are available.
- The buttons "SSH" and "RDP", if the device has these protocols.

- **DEV-001.** The page must show the device list in less than 1 second for 200 devices.
- **DEV-002.** The page must update the status of each device without a full reload.
- **DEV-003.** The operator must be able to filter the list by status, by tag and by name.
- **DEV-004.** The operator must be able to sort the list by name, by status and by uptime.
- **DEV-005.** If there are no devices, the page must show an empty state with the button "Add device".

### 6.2 Add or edit a device

The operator uses a form to add or edit a device. The form has these fields:

| Field | Type | Required | Default |
| --- | --- | --- | --- |
| Name | Text | Yes | — |
| Host | Host name or IP address | Yes | — |
| Tags | List of text | No | Empty |
| Operating system | Linux, macOS, Windows, Other | No | Other |
| SSH | On or off, port, credential | No | Off, port 22 |
| RDP | On or off, port, credential, security mode | No | Off, port 3389 |
| Checks | List of checks (Section 6.4) | No | One ICMP check |
| Metrics source | None, SSH, Proxmox API | No | None |

- **DEV-010.** The form must validate the host before the operator saves the device.
- **DEV-011.** The form must show a password field as hidden text.
- **DEV-012.** The operator must be able to use an SSH private key in place of a password.
- **DEV-013.** The operator must be able to test the connection before the operator saves the device.
- **DEV-014.** The operator must confirm before Anima deletes a device.
- **DEV-015.** When the operator deletes a device, the hub must delete its credentials, checks, heartbeats and metrics.

### 6.3 Remote access

The operator clicks "SSH" or "RDP" on a device card to start a session.

Session procedure:

1. The client sends a session request to the hub.
2. The hub reads the credential and the connection settings of the device.
3. SSH: the hub connects to the device itself (R-062, K4 Section 8). RDP and VNC: the hub opens a connection to the gateway, and the gateway connects to the device.
4. The hub sends the stream to the client through the WebSocket tunnel: terminal bytes for SSH, the Guacamole stream for RDP and VNC.
5. The client shows the session in a new tab in the main window.

- **RA-001.** The client must show each session in a tab. The operator must be able to open more than one session.
- **RA-002.** The operator must be able to move a session tab into a separate window on desktop.
- **RA-003.** An SSH session must support copy and paste, terminal resize and a scroll-back buffer.
- **RA-004.** An RDP session must change its resolution when the operator changes the window size.
- **RA-005.** An RDP session must support the clipboard in text format.
- **RA-006.** The client must send special key combinations, for example Ctrl+Alt+Delete, from a menu.
- **RA-007.** If the connection stops, the client must show the reason and a "Reconnect" button.
- **RA-008.** The hub must record the start time, the end time and the device of each session. The hub must not record the session content, except when the operator sets RA-040 on.

#### 6.3.1 SSH settings (Phase 1)

Since R-062 the hub opens the SSH connection and the client draws the terminal (K4 Section 8). The middle column names the setting as guacd called it; the hub and the client apply the same values.

| Setting | Gateway parameter | Default |
| --- | --- | --- |
| Private key and passphrase | `private-key`, `passphrase` | — |
| SSH certificate | `public-key` | — |
| Host key | `host-key` (from SEC-006) | Recorded at first connection |
| Command to run at start | `command` | Login shell |
| Keepalive interval | `server-alive-interval` | 30 seconds |
| Colour scheme | `color-scheme` | Nord (custom scheme) |
| Font and size | `font-name`, `font-size` | `OpenDyslexicM Nerd Font Mono`, 12 |
| Scroll-back lines | `scrollback` | 5000 |
| Terminal type | `terminal-type` | `xterm-256color` |

- **RA-010.** The SSH terminal uses a colour scheme from the Nord tokens in Section 5.1 (R-062: the client's xterm.js theme; before, the gateway's custom colour scheme).
- **RA-011.** The client renders SSH sessions with xterm.js, from the terminal bytes the hub relays (R-062, K4 Section 8). The terminal uses OpenDyslexicM Nerd Font Mono, which the client ships, and the Nord colours from the tokens (RA-010). The gateway image no longer needs the terminal font for SSH.

#### 6.3.2 RDP settings (Phase 2)

| Setting | Gateway parameter | Default |
| --- | --- | --- |
| Security mode | `security` | `nla` |
| Server certificate | `cert-tofu`, `cert-fingerprints` | Record at first connection |
| Ignore certificate | `ignore-cert` | Off |
| Remote Desktop Gateway | `gateway-hostname`, `gateway-port`, `gateway-username`, `gateway-password`, `gateway-domain` | Off |
| Resize method | `resize-method` | `display-update` |
| Audio out | `disable-audio` | On |
| Microphone | `enable-audio-input` | Off |
| Shared drive | `enable-drive`, `drive-path` | Off |
| RemoteApp | `remote-app`, `remote-app-dir`, `remote-app-args` | Off |
| Performance | Wallpaper, theming and GFX flags | Gateway defaults |

- **RA-020.** The client must show a warning badge on a device that uses `ignore-cert`, `security=rdp`, VNC without TLS or Telnet.
- **RA-021.** The hub must use a separate `drive-path` for each device. Do not share one drive folder between devices.

#### 6.3.3 Other remote access functions

| Function | Phase | Requirement |
| --- | --- | --- |
| VNC sessions | 2 | **RA-030.** The operator must be able to add VNC to a device. Use VNC for the graphical desktop of a Linux or macOS device (macOS Screen Sharing is VNC-compatible). The device card and header must show a "VNC" button when VNC is set up. |
| Wake-on-LAN | 2 | **RA-031.** The operator must be able to record the MAC address of a device. If the device is Down, the hub must send a Wake-on-LAN packet before it connects. |
| SFTP file transfer | 2 | **RA-032.** An SSH session must have a file panel for upload and download through SFTP. The operator must be able to set upload off and download off for each device. |
| Clipboard control | 2 | **RA-033.** The operator must be able to set copy off and paste off for each device. |
| Live sessions | 2 | **RA-034.** The client must show a list of live sessions. The operator must be able to stop a live session. |
| Session sharing | 3 | **RA-035.** The operator must be able to make a share link for a live session. A share link must be read-only by default, must expire and must stop when the owner disconnects. |
| Concurrency limit | 3 | **RA-036.** The operator must be able to set a maximum number of sessions for each device. |
| Session recording | 3 | **RA-040.** The operator must be able to set recording on for each device. It is off by default. The client must show a "Recording" badge during a recorded session. The hub must encrypt recordings at rest. |
| Recording playback | 3 | **RA-041.** The client must play recordings with `Guacamole.SessionRecording`. |
| Touch input | 5 | **RA-050.** The iOS client must have the on-screen keyboard, text input mode, touchpad mouse mode and pinch zoom. |

#### 6.3.4 Web interfaces

Many devices have a web interface for administration, for example Proxmox VE on port 8006 or Home Assistant on port 8123. The operator can open these interfaces in Anima, as with SSH, RDP and VNC.

| Field | Values | Default |
| --- | --- | --- |
| Label | Text, for example "Proxmox VE" | The host name |
| URL | An `http` or `https` URL | — |
| Open in | Anima tab, or system browser | Anima tab |

- **WEB-001.** A device can have zero or more web interfaces.
- **WEB-002.** The hub must suggest a web interface for each HTTP(S) check of the device. The operator must accept a suggestion before Anima shows it.
- **WEB-003.** The device card, the table row and the device detail header must show a "Web" button when the device has a web interface. If the device has more than one web interface, the button must open a menu with the label and the URL of each one.
- **WEB-004.** On desktop, the client must open the web interface in a session tab with an embedded Tauri web view. Do not use an `iframe`, because many interfaces do not permit embedding. The web view connects directly to the device, because the client is on the same network (Section 4.1).
- **WEB-011.** The web client (Phase 3) must open a web interface in a new browser tab. It connects directly to the device.
- **WEB-005.** The session tab must have a toolbar with: back, forward, reload, the URL, the TLS status, "Open in system browser" and "Pop out".
- **WEB-006.** The web view must use a separate data store for each device. Cookies and storage from one device must not be available to a different device or to the Anima interface.
- **WEB-007.** The web view must not have access to the Tauri commands of the client.
- **WEB-008.** For an `https` URL with a certificate that is not trusted, the client must show the SHA-256 fingerprint at the first visit. The operator must accept the certificate for this device. The client must then pin this fingerprint. If the fingerprint changes, the client must stop and show a warning, as in SEC-007.
- **WEB-009.** The audit log must record each accepted certificate.
- **WEB-010.** If the device is Down, the session tab must show an error page with the reason and a "Reload" button.

### 6.4 Uptime checks

| Check type | Test | Settings |
| --- | --- | --- |
| ICMP | Send a ping to the host | Timeout, IP version (4 or 6) |
| TCP | Open a TCP connection to a port | Port, timeout |
| TLS | Open a TLS connection to a port and examine the certificate | Port, server name, timeout |
| HTTP(S) | Send a request to a URL | URL, method, headers, body, maximum redirects, accepted status code ranges, keyword, JSON query with expected value, authentication (basic, bearer), TLS certificate check |
| DNS | Resolve a name | Name, record type, resolver, expected value |
| Push | The device sends a request to a token URL on the hub. If no request comes in the interval, the check is Down. | Interval, token |
| Group | The status of the group is the worst status of its child checks | Child checks |

Phase 2 adds these check types: Docker container, SNMP, Tailscale ping and systemd service (through SSH). Phase 3 adds domain expiry (through RDAP).

Each check has these common settings:

| Setting | Default |
| --- | --- |
| Interval | 60 seconds |
| Retries before Down | 3 |
| Retry interval | 20 seconds |
| Response time limit for Degraded | 1000 milliseconds |

- **CHK-001.** The hub must run each check at its interval.
- **CHK-002.** The hub must set the check to Down only after the number of failed retries is equal to the "Retries before Down" setting.
- **CHK-003.** The hub must record each heartbeat with the time, the status, the response time and an error message.
- **CHK-004.** The status of a device is the worst status of its checks.
- **CHK-005.** The operator must be able to pause a check.
- **CHK-006.** The operator must be able to put a device in maintenance. Anima must not show a Down status for a device in maintenance.
- **CHK-007.** For an HTTPS check, the hub must show the number of days until the TLS certificate expires.
- **CHK-008.** Each check must have an "Invert" setting. When it is on, a failed test gives the status Up. Example: a port that must be closed.
- **CHK-009.** Each check type must have a fixed identifier in the database, for example `tcp` or `tls`. Do not change the meaning of an identifier in a later version. Add a new identifier for new behaviour.
- **CHK-010.** The hub must have a watchdog for the check scheduler. If checks are late by more than 2 intervals, the hub must write an event log entry with the severity Error.
- **CHK-011.** The hub must calculate uptime from aggregate tables (DB-008). Do not read all raw heartbeats to calculate uptime.
- **CHK-012.** The hub must not start a shell command with text that the operator supplied.

### 6.5 Maintenance windows

A maintenance window is a period when Anima does not show a Down status and does not send notifications.

| Window type | Example |
| --- | --- |
| Manual | The operator starts and stops the window. |
| Single | 2026-11-20 from 22:00 to 23:30 |
| Recurring, interval | Every 14 days, for 2 hours |
| Recurring, weekday | Every Sunday from 03:00 to 04:00 |
| Recurring, day of month | The 1st day and the last day of each month |

- **MW-001.** Each window must have a time zone.
- **MW-002.** A window can continue past midnight.
- **MW-003.** The operator must be able to apply a window to one or more devices, checks or tags.
- **MW-004.** The hub must keep a window as a rule with fields. Do not keep a window as a raw cron text.

### 6.6 Resource metrics

The hub collects these metrics from each device that has a metrics source:

| Metric | Unit |
| --- | --- |
| CPU usage | Percent |
| Memory usage | Percent and bytes |
| Disk usage for each mounted file system | Percent and bytes |
| Network traffic, in and out | Bytes per second |
| System uptime | Seconds |
| Load average (Linux and macOS) | Number |
| GPU usage, for each GPU | Percent |
| Video memory (VRAM) usage, for each GPU | Percent and bytes |
| GPU temperature and power, for each GPU | Degrees Celsius and watts |

Metrics sources:

- **SSH (agentless).** The hub connects with SSH and reads system values. On Linux, the hub reads `/proc`. On macOS, the hub uses `vm_stat`, `sysctl` and `df`.
- **Proxmox API.** The hub reads the node, virtual machine and container status from the Proxmox VE API.

GPU metrics sources (through SSH):

| GPU type | Command or file | Notes |
| --- | --- | --- |
| NVIDIA | `nvidia-smi --query-gpu=index,name,utilization.gpu,memory.used,memory.total,temperature.gpu,power.draw --format=csv,noheader,nounits` | The NVIDIA driver must be installed on the device. |
| AMD | `/sys/class/drm/card*/device/gpu_busy_percent`, `mem_info_vram_used`, `mem_info_vram_total` | The `amdgpu` driver supplies these files. |
| Intel | `intel_gpu_top -J` | Integrated GPUs share system memory. Show GPU usage only. |
| Apple silicon (macOS) | None | The GPU shares system memory (unified memory). Do not show a VRAM value. Memory usage includes the GPU. |

- **MET-001.** The hub must collect metrics every 30 seconds.
- **MET-002.** The SSH metrics collector must use read-only commands only.
- **MET-003.** The device detail view must show a chart for each metric for 1 hour, 24 hours and 7 days.
- **MET-004.** The device card must show a metric in `--status-degraded` colour when the value is more than 85 percent.
- **MET-005.** Metrics for Windows devices are out of scope for Phase 2. A later version can add an agent or WinRM.
- **MET-006.** The hub must find the GPUs of a device at the first metric collection. The hub must find them again each 24 hours and after a device restart.
- **MET-007.** If a device has no GPU with its own memory, the client must not show VRAM. Do not show "0 %" for a missing GPU.
- **MET-008.** If a device has a GPU with its own memory, the device card must show a VRAM bar after the disk bar. If the device has more than one GPU, the bar must show the GPU with the highest VRAM usage.
- **MET-009.** The device detail view must show a chart for GPU usage and VRAM usage for each GPU. The chart label must include the GPU index and name.
- **MET-010.** A GPU can be passed through to a virtual machine. In this case, the hub must read the GPU metrics from the virtual machine, not from the Proxmox node.

### 6.7 Device detail view

The operator clicks a device card to open the device detail view.

- **DEV-020.** The header of the device detail view must be one line. It must show, from left to right: a "Devices" back link, the device name, the status, the host, the IP address, the device type, the operating system and the tags. The SSH, RDP and Edit buttons must be at the right end of the line.
- **DEV-021.** If the line is too long, the client must hide the end of the host and tag information with a fade. The name, the status and the buttons must always be visible. The full text must be available as a tooltip.
- **DEV-022.** The header must always show an SSH button and an RDP button. If a protocol is not set up on the device, its button must have a dashed border and a "+" icon. This button must open the Settings tab of the device. It must not start a session.

The view has these tabs:

| Tab | Content |
| --- | --- |
| Overview | Status, uptime percentage (24 hours, 7 days, 30 days), metric charts |
| Checks | The list of checks, heartbeat history, response time chart |
| Sessions | Session history for this device |
| Logs | Event log entries for this device (Section 3.8) |
| Settings | The form from Section 6.2 |

### 6.8 Notifications (Phase 3)

- **NOT-001.** The hub must send a notification when a check changes to Down and when it changes back to Up.
- **NOT-002.** The first channels must be webhook, ntfy, email (SMTP) and Apprise (through its HTTP API). Do not start the Apprise command line tool.
- **NOT-003.** The operator must be able to select the channels for each check. A channel can be "on by default" for new checks.
- **NOT-004.** Each channel must have a "Send test" button.
- **NOT-005.** The operator must be able to set a resend interval: send the notification again after each N Down heartbeats.
- **NOT-006.** The hub must send a notification when a TLS certificate expires in less than the set number of days. The default values are 21, 14 and 7 days.
- **NOT-007.** The operator must be able to change the message text with a template.

### 6.9 Integrations (Phase 3)

- **INT-001.** The hub must supply a Prometheus endpoint at `/metrics` with the status, response time, uptime ratio and certificate days of each check.
- **INT-002.** Prometheus labels must not include values that change often.
- **INT-003.** The operator must be able to make API keys for the GraphQL API. Each key must have a scope (read or write) and an expiry date.
- **INT-004.** A later version can add public status pages, incidents and status badges.

## 7. Settings page

- **SET-001.** The Settings page must be available from the navigation bar.
- **SET-002.** In Phase 1, the Settings page must show only the page title "Settings", a "Signed in as <name>" row with "Sign out" (R-048), the "Interface font" and "Monospace font" choices (UI-020), and, in the desktop app, the "Updates" row (UPD-006).

A later version adds these items to the Settings page:

- Hub address and hub connection test.
- Theme: dark or light.
- Data retention period.
- Notification channels, for example email, ntfy, Discord or Telegram.
- Default check settings.

## 8. Device management (Proxmox VE)

This function starts in Phase 4. It uses the Proxmox VE REST API (`/api2/json`) with an API token. Proxmox VE versions 8.4 and 9.x are in scope.

### 8.1 Connection

- **PVE-001.** The operator must be able to add a Proxmox VE node or cluster as a device.
- **PVE-002.** If the node is part of a cluster, the hub must add the cluster one time. Do not add each node as a separate device, because each node gives access to the full cluster.
- **PVE-003.** The hub must record the TLS certificate fingerprint at the first connection (`/nodes/{node}/certificates/info`). Do not turn off certificate verification.
- **PVE-004.** If the fingerprint changes, the hub must stop the connection and show a warning, as in SEC-007.
- **PVE-005.** The hub must read `/version` at each connection. The hub must use the correct privilege names and fields for that version.
- **PVE-006.** The hub must show the token expiry date. The hub must write an event log entry 14 days before the token expires.

### 8.2 API token privileges

Create the token with privilege separation on. Give the role on the path `/` with propagation, or on one pool.

| Role | Privileges |
| --- | --- |
| Anima read-only | `VM.Audit`, `Sys.Audit`, `Datastore.Audit`, `Pool.Audit`, `SDN.Audit`, and `VM.GuestAgent.Audit` (9.x) or `VM.Monitor` (8.x) |
| Anima power control | The read-only privileges and `VM.PowerMgmt`. Add `VM.Console` only if consoles are on. |
| Later phases | `VM.Snapshot`, `VM.Snapshot.Rollback`, `VM.Backup`, `Datastore.AllocateSpace`, `VM.Migrate`, only when the operator sets on the related function |

- **PVE-010.** The hub must not ask for `Sys.PowerMgmt` or `Sys.Modify`.
- **PVE-011.** The operator documentation must show the commands that make the two roles.

### 8.3 Phase 4 functions

| Function | API endpoints | Requirement |
| --- | --- | --- |
| Inventory | `/cluster/resources`, `/cluster/status` | **PVE-020.** The hub must read all nodes, virtual machines, containers and storage with one call. The hub must show virtual machines and containers as child devices of the cluster. |
| Status | `/nodes/{node}/{type}/{id}/status/current` | **PVE-021.** The hub must show the state, uptime, lock and HA state of each guest. |
| Metrics | `/cluster/metrics/export`, `/nodes/{node}/{type}/{id}/rrddata` | **PVE-022.** The hub must collect metrics with `/cluster/metrics/export`. Use `rrddata` only for history that the hub does not have. Do not poll each guest separately. |
| Pressure metrics | `rrddata` (9.x only) | **PVE-023.** On 9.x, the hub must show CPU, memory and IO pressure (PSI). |
| Guest agent | `.../qemu/{id}/agent/network-get-interfaces`, `get-fsinfo`, `get-osinfo`; `.../lxc/{id}/interfaces` | **PVE-024.** The hub must find the IP addresses of each guest. The SSH and RDP buttons must use these addresses. If the agent is not installed, the status is Unknown. It is not an error. |
| Power control | `.../status/start`, `shutdown`, `stop`, `reboot` | **PVE-025.** The operator must be able to start, shut down, stop and restart a guest. The operator must confirm before a stop or a restart. |
| Tasks | `/cluster/tasks`, `/nodes/{node}/tasks/{upid}/status`, `.../log` | **PVE-026.** Each write call gives a task ID (UPID). The hub must wait for the task result and write it to the event log. An HTTP 200 result only means that the task started. |
| Tags | Guest `tags` field | **PVE-027.** The hub must show Proxmox tags as Anima tags. |
| HA | `/cluster/ha/status/current`, `/cluster/ha/resources` | **PVE-028.** The client must show an HA badge on a guest that HA manages. Before a stop, the client must tell the operator that HA can start the guest again. |
| Storage | `/nodes/{node}/storage` | **PVE-029.** The hub must show the usage of each storage. The usage limit for Degraded is 85 percent. |

### 8.4 Consoles

The gateway cannot open a Proxmox console. Proxmox sends VNC and terminal data over a WebSocket, and the VNC port is only available on the node. Thus consoles use a separate path.

- **PVE-030.** The hub must relay the console WebSocket. The hub must call `vncproxy` or `termproxy` and then connect to `vncwebsocket` at once, because the ticket expires after about 10 seconds.
- **PVE-031.** The client must use noVNC for a virtual machine console. The client must use xterm.js for a container console and a node shell.
- **PVE-032.** Test if an API token can open a console. If it cannot, the hub must use a separate console user and renew its ticket every 90 minutes.
- **PVE-033.** For a guest with an IP address, SSH and RDP through the gateway are the preferred methods. The console is for a guest without network access.

### 8.5 Later functions (Phase 6 or later)

| Function | Endpoints | Priority |
| --- | --- | --- |
| Snapshots: make, list, roll back | `.../snapshot`, `.../snapshot/{name}/rollback` | Should |
| Backups: last backup age, start a backup, guests without a backup | `/nodes/{node}/vzdump`, `/cluster/backup`, `/cluster/backup-info/not-backed-up` | Should |
| Bulk start and stop by tag (9.x) | `/cluster/bulk-action/guest/...` | Should |
| Proxmox sends events to the hub (webhook) | `/cluster/notifications/endpoints/webhook`, `/matchers` | Should |
| Clone from a template, cloud-init with SSH keys | `.../clone`, `.../template`, `.../cloudinit` | Could |
| Migration to a different node | `.../migrate` | Could |
| Pending updates and disk SMART status | `/nodes/{node}/apt/update`, `/nodes/{node}/disks/smart` | Could |
| SPICE: download a `.vv` file for `remote-viewer` | `.../spiceproxy` | Could |
| Firewall, SDN and replication status | `/cluster/firewall`, `/cluster/sdn`, `/cluster/replication` | Could |

### 8.6 Other hypervisors

A later version can manage libvirt hosts and Docker hosts. Prepare for this now:

- **PVE-040.** The hub must use one provider interface for inventory, power control, metrics, consoles and snapshots. Each provider must declare the functions that it supports.
- **PVE-041.** The database must keep a provider task ID as opaque text (`external_task_id`). Do not use a Proxmox-only column type.

## 9. Delivery phases

| Phase | Content | Platforms |
| --- | --- | --- |
| 1 | Repository structure and quality gates (Sections 12 and 13). VPN setup: Headscale on the VPS, subnet routers, clients (Section 4.2). Design token build (Section 5.5), fonts and icons, motion helpers. Application shell, navigation bar, Nord theme, empty Settings page. Hub with authentication, GraphQL API, PostgreSQL and the three logs. Device list, add and edit device, tags. ICMP, TCP, TLS, HTTP(S), DNS, push and group checks. Maintenance windows. SSH sessions. | macOS, Linux |
| 2 | RDP and VNC sessions. Web interfaces. Wake-on-LAN. SFTP. Live session list. SSH metrics. More check types. Device detail view with charts. | macOS, Linux |
| 3 | Web client. Notifications. Prometheus endpoint and API keys. Session sharing and recording. Settings page items. | macOS, Linux, Web |
| 4 | Proxmox VE integration (Sections 8.1 to 8.4). | macOS, Linux, Web |
| 5 | iOS client. | All |
| 6 | Proxmox later functions (Section 8.5). Other hypervisors. Site agents for Wake-on-LAN and local checks. Traffic data (Section 4.3). | All |

### 9.1 Phase 1 acceptance criteria

1. The operator can start the hub, the gateway and PostgreSQL with one Docker Compose command.
2. A Mac at one site can open an SSH session to a device at a different site through the VPN. The Mac does not need a manual VPN connection step.
3. The operator can open the desktop client on macOS and on Linux.
4. The client shows the navigation bar with "Devices" and "Settings".
5. The Settings page shows an empty state.
6. The operator can add, edit and delete a device.
7. The device card shows the correct status within 2 check intervals after a device stops.
8. The operator can open an SSH session to a Linux device from the device card.
9. All colours in the interface come from the Nord tokens in Section 5.1.
10. The client gets all data through the GraphQL API. The device status changes on the screen through a subscription.
11. The audit log records each add, change and delete of a device.
12. No log contains a password, a private key or a token.
13. All colours, spaces, font sizes and motion values come from tokens. The token gate (TOK-003) passes.
14. Each motion role in Section 5.7.3 that Phase 1 uses has a reduced-motion form. The Phase 1 motion audit (Section 5.8) has no open finding of the classes "layout jump" or "wrong motion".
15. The CUPID gates (CQ-010, CQ-040, CQ-043) and the import graph check (REPO-004) pass in the gate script (`bun run gates`). The baseline file is empty.
16. A merge to `main` publishes a release. An installed earlier build downloads it, shows the restart banner, and runs the new version after "Restart to update". A feed with a bad signature is refused and the app continues to run (UPD-001–012).

## 10. iOS constraints

These constraints apply to Phase 5. Record them now so that the earlier phases do not block them.

- iOS stops network connections when the application goes to the background. A session must reconnect when the operator opens the application again.
- iOS does not let the application run checks in the background. This is not a problem, because the hub runs all checks.
- The RDP session needs a software keyboard with special keys, for example Ctrl, Alt, Esc and the function keys.
- The interface must use the bottom navigation bar from Section 5.3.

## 11. Open decisions

| Number | Decision | Recommendation |
| --- | --- | --- |
| OD-1 | User interface framework | SvelteKit in static mode. It makes small bundles and works well with Tauri. React is also acceptable. |
| OD-2 | Hub authentication method | One operator account with a password and a TOTP second factor. Add OIDC in a later version. |
| OD-3 | Desktop client without a hub | Not supported. A later version can add direct SSH from the Rust layer for offline use. |
| OD-4 | Hub on Proxmox | The operator can run the hub in an LXC container on the Proxmox node. |
| OD-5 | Time-series storage | Start with plain PostgreSQL and daily partitions. Add the TimescaleDB extension only if metric queries become slow. |
| OD-7 | Self-signed certificates in the Tauri web view | Test certificate pinning on macOS (WKWebView) and Linux (WebKitGTK) before Phase 2. If the web view cannot pin a certificate, open the interface in the system browser and show the pinned fingerprint next to the button. |
| OD-8 | Spring library | `motion` (vanilla API). It re-targets springs from the current velocity and works with Svelte and React. |
| OD-9 | Hub location | Hostinger VPS. The hub continues to operate when a site loses power or internet, and it can report that the site is down. Risk: a VPS breach exposes the device credentials (SEC-003 and SEC-004 decrease this risk). Alternative: the hub at home, and a small outside checker on the VPS. |
| OD-10 | Headscale or the Tailscale hosted control server | Headscale, because it keeps the control server under the control of the operator. Headscale does not have "Tailnet Lock" (devices must approve each new node). The Tailscale hosted plan has it. Verify this before Phase 1. |
| OD-11 | Gateway location | The gateway runs with the hub on the VPS. A session to a device at a site then goes VPS to site. If the latency is too high, a later version can run a gateway at each site (Guacamole supports a gateway for each connection). |
| OD-6 | Central log search | Not necessary now. A later version can send the process log to Loki or a similar system. |

## 12. Code quality (CUPID)

Anima uses the CUPID properties for code quality: Composable, Unix philosophy, Predictable, Idiomatic and Domain-based (https://cupid.dev/properties/). A property is a direction, not a rule. The numeric limits in Section 12.2 show where to look. They do not decide the fix. Do not make a change only to satisfy a number.

### 12.1 The five properties in Anima

| Property | Question | Anima examples |
| --- | --- | --- |
| Composable | Can a caller use this unit from its name and signature only, without many dependencies? | The Guacamole protocol code is a library with no I/O (`crates/guac-protocol`). A check type takes a probe result and gives a status. It does not read the database. |
| Unix philosophy | Can you state the purpose of the unit in one sentence without "and"? | The check scheduler schedules. It does not probe, store or notify. Those are separate stages. |
| Predictable | Does it do what its name says, each time? Can you see why it failed? | Maintenance window evaluation is a pure function that receives the current time. Errors are typed. Logs are at the I/O boundaries (LOG-001). |
| Idiomatic | Does it read like good code in the language, and like the code next to it? | Rust: rustfmt and Clippy `pedantic`. TypeScript and Svelte: typescript-eslint `strict` and Svelte 5 runes. One GraphQL client, one spring helper, one error type for each domain. |
| Domain-based | Would an operator recognise the names and the folders? | Folders are `devices/`, `checks/`, `sessions/`, `metrics/`. Types are `DeviceId`, `Fingerprint`, `Percent`, `Bytes`. |

- **CQ-001.** Each pull request must answer the review checklist in Section 12.4.
- **CQ-002.** Each finding in a review must name the property that it serves.
- **CQ-003.** Do not add an abstraction (an interface, a trait or a factory) that has only one implementation and one caller. This is surface area, not composability.
- **CQ-004.** Do not split code that always changes together, for example a type and its parser, or a component and its styles.

### 12.2 Measurable limits

| Signal | Property | Limit |
| --- | --- | --- |
| Code lines in a file (no blank lines, no comments) | U | 300 |
| Lines in a function | U | 60 |
| Parameters of a function | C | 4 (Rust: Clippy default 7 for `too_many_arguments`) |
| Cyclomatic complexity | U | 15 |
| Nesting depth | U | 4 |
| Files in a folder | D | 12 |
| Exports that no other module uses | C | 0 |
| Import cycles | C | 0 |
| Weak names in paths and types (see CQ-030) | D | 0 |
| Swallowed errors, `any`, unchecked casts, panics on input | P | 0 |
| Clock, random, environment or global reads in core modules | P | 0 |
| Suppressed lints without a reason | I | 0 |

- **CQ-010.** The gate script (`bun run gates`) must measure each signal in this table. It must fail when a new file or a changed file is more than a limit. The gates run on the machine that makes the change, before each review and each merge. Do not run gates or tests in GitHub Actions. GitHub Actions is only for deployment.
- **CQ-011.** The repository starts with an empty baseline file (`quality/baseline.json`). An entry records a file and its current value. An entry can only decrease. Do not add an entry by hand. Do not increase a value to make a change pass. Split the code.
- **CQ-012.** Generated files (tokens, GraphQL types, migrations) are not measured. List them in `quality/generated.txt`.
- **CQ-013.** Suppress a lint only with a reason. Rust: `#[expect(clippy::rule, reason = "...")]`. TypeScript: `// eslint-disable-next-line rule -- reason`.

### 12.3 Rules for the code

Composable and Predictable:

- **CQ-020.** Keep a pure core and put I/O at the edge. Core logic receives data and returns data. It does not read the database, the network, the clock, random values or environment variables.
- **CQ-021.** Pass time, randomness and configuration in as parameters. This applies to the check scheduler, maintenance windows, uptime calculation, aggregation and retention.
- **CQ-022.** Each Rust domain has a typed error (`thiserror`). The hub converts domain errors to GraphQL errors at the API boundary (API-010). Do not use `anyhow` in domain code.
- **CQ-023.** Use `unwrap` and `expect` only for an invariant that the code itself makes. The `expect` message must state the invariant. Never use them on input, environment or I/O.
- **CQ-024.** Do not swallow an error. Do not use `catch {}`, `let _ = result` or an empty `else` for an error.
- **CQ-025.** In TypeScript, do not use `any` or the non-null operator `!`. Parse unknown data at the boundary.
- **CQ-026.** Put logs and metrics at the I/O boundaries only: API requests, database calls, probes, gateway connections and VPN state.

Domain-based:

- **CQ-030.** Do not use these words in a file, folder or type name: `Manager`, `Service`, `Helper`, `Handler`, `Processor`, `Factory`, `Util`, `data`, `info`, `item`, `misc`, `common`. Do not make the folders `utils/`, `helpers/`, `common/`, `components/`, `services/` or `models/`.
- **CQ-031.** Use domain types for values that are easy to confuse: `DeviceId`, `CheckId`, `SessionId`, `SiteId`, `Percent`, `Bytes`, `Fingerprint`, `HostKey`, `Cidr`, `Upid`.
- **CQ-032.** Each domain owns its tables. Only the `store` module of a domain sends SQL to its tables. Other domains call the public functions of that domain.
- **CQ-033.** Each domain gives its GraphQL part (queries, mutations, subscriptions) in its own `graphql.rs`. The schema root only joins these parts.

Idiomatic:

- **CQ-040.** Rust: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` with `clippy::pedantic` in `[workspace.lints]`, and one module layout (`foo.rs` with `foo/`) enforced by `clippy::mod_module_files`.
- **CQ-041.** Rust visibility is private by default. Use `pub(crate)` for use between domains. Use `pub` only for the public interface of a library crate.
- **CQ-042.** Rust integration tests are in one crate for each binary (`tests/it/main.rs`) with modules that follow `src/`.
- **CQ-043.** TypeScript and Svelte: ESLint `recommended`, typescript-eslint `strict` and `stylistic`, eslint-plugin-svelte `recommended`, `svelte-check`. Use Svelte 5 runes. Use `$derived` for computed values. Use `$effect` only for real side effects.
- **CQ-044.** A route composes components. It does not contain logic. Logic (filters, sorts, formats, date calculation) goes in a `.ts` module next to the component.
- **CQ-045.** API access goes through one typed GraphQL client for each domain (`$lib/<domain>/api.ts`). Do not call `fetch` in components.
- **CQ-046.** Tests follow the `test-audit` rules. A test must test behaviour, not the structure of the code. If a move of code makes a test fail, the test is wrong.

### 12.4 Review checklist

- **C:** Does each export have a user outside its module? Can a caller use it from the signature only? Is I/O at the edge?
- **U:** Can you state the purpose of each new or changed unit in one sentence without "and"?
- **P:** Does each name tell the truth? Is the core deterministic? Do errors come back as typed results, with logs at the boundaries?
- **I:** Does it read like standard code in the language, and like the code next to it?
- **D:** Do the folder, file and type names come from the domain? Can a new developer find it?
- Did the change make a baseline file larger, or add a suppression without a reason?

### 12.5 Quality audits

- **CQ-050.** At the end of each phase, run the `code-quality` audit workflow on the repository, in audit mode. Apply only the items that the operator accepts. Each item is a separate branch and pull request.
- **CQ-051.** When you fix a failed limit, move code without a rewrite. Find the seam first: a domain concept, pure logic against I/O, a user interface section, or a pipeline stage.

## 13. Repository structure

The repository is a monorepo. The structure follows the domain (CQ-030), not technical layers. The user interface structure assumes SvelteKit (OD-1).

```text
anima/
├── AGENTS.md                  Agent and contributor rules (Section 12 summary)
├── CLAUDE.md                  Points to AGENTS.md
├── Cargo.toml                 Rust workspace, [workspace.lints] (CQ-040)
├── package.json               JavaScript workspace (pnpm)
├── contracts/                 Frozen interfaces for parallel work (implementation plan, Section 4)
├── docs/
│   ├── design-spec.md         This document
│   ├── implementation-plan.md Agent implementation plan
│   ├── programme/             Programme file and package plans
│   ├── decisions/             Architecture decision records, one file for each decision
│   ├── mockup/                Interactive mockup
│   └── research/              Research notes, motion audits (MV-007)
├── design/
│   └── tokens/                Design token source (TOK-001)
├── quality/
│   ├── baseline.json          Shrink-only baseline (CQ-011)
│   └── generated.txt          Generated files that are not measured (CQ-012)
├── crates/
│   ├── hub/                   The hub binary (anima-hub)
│   │   ├── src/
│   │   │   ├── main.rs        Start-up and wiring only
│   │   │   ├── schema.rs      GraphQL schema root: joins the domain parts (CQ-033)
│   │   │   ├── access/        Login, tokens, API keys, permitted networks (NET-001)
│   │   │   ├── devices/       Device inventory, sites, tags
│   │   │   ├── checks/        Check types, scheduler, heartbeats, watchdog
│   │   │   ├── maintenance/   Maintenance windows (pure rules + store)
│   │   │   ├── metrics/       Collection (SSH, GPU, Proxmox), aggregates, retention
│   │   │   ├── sessions/      Session start, tunnel, history, sharing
│   │   │   ├── credentials/   Encryption and storage of credentials
│   │   │   ├── web_interfaces/ Web interface records and suggestions
│   │   │   ├── events/        Event log and audit log
│   │   │   ├── notifications/ Channels and rules (Phase 3)
│   │   │   ├── proxmox/       Proxmox VE provider (Phase 4)
│   │   │   └── traffic/       DNS and flow data (Phase 6)
│   │   ├── migrations/        sqlx migrations
│   │   └── tests/it/          Integration tests (CQ-042)
│   ├── guac-protocol/         Guacamole protocol: parse and encode, no I/O
│   └── theme/                 Generated Rust tokens (terminal colour scheme)
├── apps/
│   ├── client/                SvelteKit user interface, used by desktop and web
│   │   └── src/
│   │       ├── routes/
│   │       │   ├── devices/           Devices page
│   │       │   ├── devices/[id]/      Device detail view
│   │       │   └── settings/          Settings page
│   │       ├── lib/
│   │       │   ├── devices/   Device card, table, filters, api.ts
│   │       │   ├── checks/    Heartbeat bar, check editor, api.ts
│   │       │   ├── metrics/   Metric bars, GPU panel
│   │       │   ├── sessions/  Session tabs: terminal, remote desktop, web view
│   │       │   ├── events/    Logs panel
│   │       │   ├── hub/       GraphQL connection, subscriptions, network status
│   │       │   ├── charts/    Chart primitives
│   │       │   ├── forms/     Form primitives and validation
│   │       │   ├── icons/     Nerd Font icon table (UI-016)
│   │       │   └── motion/    Springs, motion roles, reduced motion (MOT-041)
│   │       └── styles/        Generated tokens.css, fonts
│   └── desktop/
│       └── src-tauri/         Tauri shell: windows, key store, web views (WEB-004)
└── deploy/
    ├── compose.yaml           Hub, gateway, PostgreSQL (DB-007 backup)
    ├── gateway/               guacd image with fonts (GW-001; RDP and VNC)
    └── headscale/             Headscale configuration and ACLs (VPN-004)
```

Each hub domain folder has the same shape. Add only the files that the domain needs:

| File | Content | I/O |
| --- | --- | --- |
| `<domain>.rs` | Public interface of the domain: types and functions that other domains can use | No |
| `model.rs` | Domain types | No |
| `rules.rs` (or a name for the concept, for example `schedule.rs`) | Pure logic | No |
| `store.rs` | SQL for the tables of the domain | Database |
| `graphql.rs` | Queries, mutations and subscriptions of the domain | API |
| `tests.rs` | Unit tests | No |

- **REPO-001.** A new domain is a new folder with a noun from the domain. Do not add a folder named for a technical layer at the top of `src/`.
- **REPO-002.** `crates/hub/src/main.rs` and `schema.rs` contain wiring only. They contain no business rules.
- **REPO-003.** The Tauri shell (`apps/desktop/src-tauri`) contains only functions that need the operating system: windows, the key store and web views. All other logic is in the hub or the client.
- **REPO-004.** A domain must not import the `store` module of a different domain (CQ-032). The gate script must check the import graph for this rule and for cycles.
- **REPO-005.** A folder with more than 12 files must be split into sub-domains, for example `checks/types/`. A Rust module file `name.rs` that has a folder `name/` belongs to that folder and does not count in its parent folder.

## 14. References

| Subject | Source |
| --- | --- |
| Guacamole connection parameters | https://guacamole.apache.org/doc/gug/configuring-guacamole.html |
| Guacamole protocol and handshake | https://guacamole.apache.org/doc/gug/guacamole-protocol.html |
| Guacamole recording and playback | https://guacamole.apache.org/doc/gug/recording-playback.html |
| Guacamole 1.6.0 release notes | https://guacamole.apache.org/releases/1.6.0/ |
| Uptime Kuma monitor types | https://github.com/louislam/uptime-kuma/tree/master/server/monitor-types |
| Uptime Kuma aggregate tables (uptime calculation) | https://github.com/louislam/uptime-kuma/pull/2750 |
| Uptime Kuma maintenance windows | https://github.com/louislam/uptime-kuma/blob/master/server/model/maintenance.js |
| Uptime Kuma v1 to v2 migration lessons | https://github.com/louislam/uptime-kuma/wiki/Migration-From-v1-To-v2 |
| Nerd Fonts (OpenDyslexic patched font, icon cheat sheet) | https://www.nerdfonts.com/ and https://github.com/ryanoasis/nerd-fonts/tree/master/patched-fonts/OpenDyslexic |
| CUPID properties | https://cupid.dev/properties/ |
| Proxmox VE API viewer | https://pve.proxmox.com/pve-docs/api-viewer/ |
| Proxmox VE users, roles and tokens | https://pve.proxmox.com/pve-docs/chapter-pveum.html |
| Proxmox VE roadmap (version changes) | https://pve.proxmox.com/wiki/Roadmap |

Items that are not yet verified:

- If a Proxmox API token can open a console (PVE-032).
- The exact names of the Proxmox 9.x pressure metric fields (PVE-023).
- The privileges in the built-in `PVEAuditor` role on Proxmox 9.x.
