//! The limits of the proxy. Before a name is known: the connections open in all and for each
//! client network (a /64 of IPv6, an address of IPv4), and the times that a connection and a
//! request may take (SF-NEW-revroutelimits). For each
//! name (Section 12.7): connections at one time, new connections each second, and bytes in each
//! period. The numbers are parameters: the operator decides them with the prices (OD-HOST-40).
//! Time is a parameter too, in milliseconds from any fixed start. No I/O.

use std::{
    collections::HashMap,
    net::{IpAddr, Ipv6Addr},
    sync::{Arc, Mutex, PoisonError},
    time::Duration,
};

/// The times after which the proxy ends a connection that does not go on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Timeouts {
    /// From the accept to the end of the TLS handshake, the PROXY header of the door included.
    pub handshake: Duration,
    /// For the whole head of a request.
    pub head: Duration,
    /// For the head and the body of a request together, from the first byte of the head: a
    /// client that sends one byte before each idle time ends cannot hold a stream for longer.
    pub request: Duration,
    /// With no byte in either direction on one stream.
    pub idle: Duration,
}

impl Default for Timeouts {
    fn default() -> Self {
        Self {
            handshake: Duration::from_secs(10),
            head: Duration::from_secs(10),
            request: Duration::from_secs(60),
            idle: Duration::from_secs(300),
        }
    }
}

/// The connections that one listener keeps open before it knows a name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Caps {
    /// In all.
    pub total: u32,
    /// For each client network ([`client_key`]). Below the streams of one link of the hub
    /// (`ANIMA_ROUTES_STREAMS_PER_LINK`, 32), so one client cannot hold each stream of a device.
    pub per_client: u32,
}

/// The key of a client for its cap: the /64 of an IPv6 address, which one host gets for itself,
/// and the whole address of IPv4 (/32). An IPv4 address in IPv6 form is IPv4.
#[must_use]
pub fn client_key(client: IpAddr) -> IpAddr {
    match client.to_canonical() {
        IpAddr::V6(address) => {
            let network = u128::from(address) & !((1_u128 << 64) - 1);
            IpAddr::V6(Ipv6Addr::from(network))
        }
        v4 @ IpAddr::V4(_) => v4,
    }
}

/// The connections open now on one listener, in all and for each client network.
#[derive(Debug)]
pub struct Gate {
    caps: Caps,
    total: u32,
    clients: HashMap<IpAddr, u32>,
}

impl Gate {
    #[must_use]
    pub fn new(caps: Caps) -> Self {
        Self {
            caps,
            total: 0,
            clients: HashMap::new(),
        }
    }

    /// Admits one more connection in all; `false` at the cap.
    pub fn enter(&mut self) -> bool {
        if self.total >= self.caps.total {
            return false;
        }
        self.total += 1;
        true
    }

    /// A connection that [`Gate::enter`] admitted ended.
    pub fn leave(&mut self) {
        self.total = self.total.saturating_sub(1);
    }

    /// Admits one more connection of `client`; `false` at the cap of its network.
    pub fn enter_client(&mut self, client: IpAddr) -> bool {
        let open = self.clients.entry(client_key(client)).or_default();
        if *open >= self.caps.per_client {
            return false;
        }
        *open += 1;
        true
    }

    /// A connection that [`Gate::enter_client`] admitted ended.
    pub fn leave_client(&mut self, client: IpAddr) {
        let key = client_key(client);
        if let Some(open) = self.clients.get_mut(&key) {
            *open = open.saturating_sub(1);
            if *open == 0 {
                self.clients.remove(&key);
            }
        }
    }
}

/// One place under a [`Gate`] until it is dropped: in all, and for its client once known.
pub struct Pass {
    gate: Arc<Mutex<Gate>>,
    client: Option<IpAddr>,
}

impl Pass {
    /// A place in all; `None` at the cap.
    #[must_use]
    pub fn enter(gate: &Arc<Mutex<Gate>>) -> Option<Self> {
        let entered = gate.lock().unwrap_or_else(PoisonError::into_inner).enter();
        entered.then(|| Self {
            gate: Arc::clone(gate),
            client: None,
        })
    }

    /// Takes a place for `client`; `false` at its cap.
    pub fn admit(&mut self, client: IpAddr) -> bool {
        let mut gate = self.gate.lock().unwrap_or_else(PoisonError::into_inner);
        let admitted = gate.enter_client(client);
        if admitted {
            self.client = Some(client);
        }
        admitted
    }
}

impl Drop for Pass {
    fn drop(&mut self) {
        let mut gate = self.gate.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(client) = self.client {
            gate.leave_client(client);
        }
        gate.leave();
    }
}

/// The limits of one name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    /// Connections open at one time.
    pub connections: u32,
    /// New connections each second, with a burst of the same number.
    pub new_per_second: u32,
    /// Bytes in both directions in each period; 0: no limit.
    pub bytes_per_period: u64,
    /// The length of a period, in milliseconds.
    pub period_ms: u64,
}

/// Why a connection was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    Connections,
    Rate,
    Bytes,
}

#[derive(Clone, Copy, Debug, Default)]
struct State {
    open: u32,
    /// New connections that the bucket still holds, in thousandths.
    tokens_milli: u64,
    refilled_ms: u64,
    period_start_ms: u64,
    bytes: u64,
    started: bool,
}

/// The state of the limits of each name.
#[derive(Debug)]
pub struct Limiter {
    limits: Limits,
    names: HashMap<String, State>,
}

impl Limiter {
    #[must_use]
    pub fn new(limits: Limits) -> Self {
        Self {
            limits,
            names: HashMap::new(),
        }
    }

    /// Admits one new connection to `name` at `now_ms`, or says which limit refuses it.
    ///
    /// # Errors
    /// The [`Refusal`] of the first limit that is reached.
    pub fn admit(&mut self, name: &str, now_ms: u64) -> Result<(), Refusal> {
        let limits = self.limits;
        let state = self.state(name, now_ms);
        if state.open >= limits.connections {
            return Err(Refusal::Connections);
        }
        if limits.bytes_per_period > 0 && state.bytes >= limits.bytes_per_period {
            return Err(Refusal::Bytes);
        }
        let elapsed = now_ms.saturating_sub(state.refilled_ms);
        let full = u64::from(limits.new_per_second) * 1000;
        state.tokens_milli =
            (state.tokens_milli + elapsed * u64::from(limits.new_per_second)).min(full);
        state.refilled_ms = now_ms;
        if state.tokens_milli < 1000 {
            return Err(Refusal::Rate);
        }
        state.tokens_milli -= 1000;
        state.open += 1;
        Ok(())
    }

    /// A connection to `name` ended.
    pub fn release(&mut self, name: &str) {
        if let Some(state) = self.names.get_mut(name) {
            state.open = state.open.saturating_sub(1);
        }
    }

    /// Adds `bytes` to the period of `name`; whether the name is still under its limit.
    pub fn add_bytes(&mut self, name: &str, bytes: u64, now_ms: u64) -> bool {
        let limit = self.limits.bytes_per_period;
        let state = self.state(name, now_ms);
        state.bytes = state.bytes.saturating_add(bytes);
        limit == 0 || state.bytes < limit
    }

    /// The state of `name`, with a new period when the last one ended.
    fn state(&mut self, name: &str, now_ms: u64) -> &mut State {
        let limits = self.limits;
        let state = self.names.entry(name.to_owned()).or_default();
        if !state.started {
            *state = State {
                tokens_milli: u64::from(limits.new_per_second) * 1000,
                refilled_ms: now_ms,
                period_start_ms: now_ms,
                started: true,
                ..State::default()
            };
        }
        if now_ms.saturating_sub(state.period_start_ms) >= limits.period_ms {
            state.period_start_ms = now_ms;
            state.bytes = 0;
        }
        state
    }
}
