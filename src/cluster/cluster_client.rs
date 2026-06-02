/// RedisClusterClient — multi-node cluster client with hash-slot routing.
///
/// `RedisClusterClient` wraps `Arc<RefCell<ClusterInner>>` so multiple
/// coroutines can share the same cluster connections. It routes each
/// command to the correct node based on `CRC16(key) % 16384`.
///
/// # Example
///
/// ```no_run
/// use may_redis::RedisClusterClient;
///
/// let cluster = may::run(|| {
///     may::go(|| {
///         let client = RedisClusterClient::connect(&[
///             "192.168.1.10:6379",
///             "192.168.1.11:6379",
///             "192.168.1.12:6379",
///         ]).join();
///     }).join()
/// });
/// ```
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use may::sync::spsc;

use crate::cluster::compute_slot;
use crate::cluster::slot_map::{NodeId, SlotMap};
use crate::connection::{Connection, ConnectionLimitError, Request};
use crate::core::{FromRedisValue, RedisError, RedisValue};
use crate::protocol::builder::CommandBuilder;

// ---------------------------------------------------------------------------
// SeedNode
// ---------------------------------------------------------------------------

/// A seed node for initial topology discovery.
///
/// The cluster client connects to one seed node, queries `CLUSTER NODES`,
/// then builds the full slot map from the response.
#[derive(Debug, Clone)]
pub struct SeedNode {
    /// Host or IP address.
    pub host: String,
    /// Redis port.
    pub port: u16,
}

impl SeedNode {
    /// Parse a `host:port` string into a SeedNode.
    ///
    /// # Errors
    /// Returns [`RedisError::Parse`] if the string is not `host:port`.
    pub fn parse(s: &str) -> Result<Self, RedisError> {
        let (host, port_str) = s.rsplit_once(':').ok_or_else(|| {
            RedisError::Parse(format!(
                "invalid seed node address: {s} (expected host:port)"
            ))
        })?;
        let port: u16 = port_str.parse().map_err(|_| {
            RedisError::Parse(format!("invalid port in seed node address: {s}"))
        })?;
        Ok(Self {
            host: host.to_string(),
            port,
        })
    }
}

// ---------------------------------------------------------------------------
// RefreshPolicy
// ---------------------------------------------------------------------------

/// Policy for automatic topology refresh.
#[derive(Debug, Clone)]
pub enum RefreshPolicy {
    /// Refresh every N seconds.
    Periodic(Duration),
    /// Never refresh automatically (admin must call manually).
    Manual,
    /// Refresh on cluster errors and periodically.
    OnErrorAndPeriodic(Duration),
}

impl RefreshPolicy {
    /// Returns the interval for periodic refresh, or `None` if manual.
    #[must_use]
    pub const fn interval(&self) -> Option<Duration> {
        match self {
            Self::Periodic(d) | Self::OnErrorAndPeriodic(d) => Some(*d),
            Self::Manual => None,
        }
    }

    /// Returns `true` if this policy triggers refresh on errors.
    #[must_use]
    pub const fn on_error(&self) -> bool {
        matches!(self, Self::OnErrorAndPeriodic(_))
    }
}

// ---------------------------------------------------------------------------
// ClusterInner
// ---------------------------------------------------------------------------

/// Internal cluster state shared across coroutines.
///
/// Wrapped in `Arc<RefCell<...>>` for interior mutability. `Connection`
/// is stored behind `Arc` so multiple coroutines can share the same
/// connection handle without holding a borrow-guard.
#[allow(dead_code)]
pub(crate) struct ClusterInner {
    /// Slot → NodeId mapping.
    pub slot_map: SlotMap,
    /// NodeId → Connection (Arc for sharing across coroutines).
    pub connections: HashMap<NodeId, Arc<Connection>>,
    /// Seed nodes for topology discovery.
    pub seed_nodes: Vec<SeedNode>,
    /// Refresh policy.
    pub refresh_policy: RefreshPolicy,
}

impl ClusterInner {
    /// Create a new empty cluster inner.
    #[must_use]
    pub fn new(seed_nodes: Vec<SeedNode>, refresh_policy: RefreshPolicy) -> Self {
        Self {
            slot_map: SlotMap::empty(),
            connections: HashMap::new(),
            seed_nodes,
            refresh_policy,
        }
    }

    /// Look up the connection for a given slot.
    ///
    /// # Errors
    /// Returns [`RedisError::Parse`] if the slot is unassigned.
    pub fn connection_for_slot(
        &self,
        slot: u16,
    ) -> Result<Arc<Connection>, RedisError> {
        let node_id = self
            .slot_map
            .node_for_slot(slot)
            .ok_or_else(|| RedisError::Parse(format!("unknown slot {slot}")))?;
        self.connections
            .get(&node_id)
            .ok_or_else(|| {
                RedisError::Parse(format!("no connection for node {node_id}"))
            })
            .cloned()
    }
}

// ---------------------------------------------------------------------------
// RedisClusterClient
// ---------------------------------------------------------------------------

/// Main entry point for Redis Cluster operations.
///
/// `RedisClusterClient` wraps `Arc<RefCell<ClusterInner>>` so multiple
/// coroutines can share the same cluster connections. It implements
/// hash-slot-based routing to the correct node for every command.
#[derive(Clone)]
pub struct RedisClusterClient {
    pub(crate) inner: std::rc::Rc<RefCell<ClusterInner>>,
}

impl RedisClusterClient {
    /// Connect to a Redis Cluster given seed node addresses.
    ///
    /// For each seed node, the client attempts to connect and query
    /// `CLUSTER NODES` to discover the full topology. It stops on the
    /// first seed that returns a valid slot map.
    ///
    /// # Arguments
    /// * `seeds` — Slice of `"host:port"` strings for seed nodes.
    ///
    /// # Errors
    /// Returns [`RedisError`] if no seed node responds with a valid
    /// cluster topology.
    ///
    /// # May runtime requirement
    /// This function requires the may coroutine runtime. Call inside
    /// `may::run(|| { may::go! { ... }.join() })`.
    pub fn connect(seeds: &[&str]) -> Result<Self, RedisError> {
        let seed_nodes: Vec<SeedNode> = seeds
            .iter()
            .map(|s| SeedNode::parse(s))
            .collect::<Result<Vec<_>, _>>()?;

        let inner_rc = std::rc::Rc::new(RefCell::new(ClusterInner::new(
            seed_nodes,
            RefreshPolicy::OnErrorAndPeriodic(Duration::new(60, 0)),
        )));

        // Collect host:port pairs to avoid borrow issues.
        let seeds_to_try: Vec<_> = inner_rc
            .borrow()
            .seed_nodes
            .iter()
            .map(|s| (s.host.clone(), s.port))
            .collect();

        for (host, port) in seeds_to_try {
            let mut inner = inner_rc.borrow_mut();
            if Self::discover_seed(&host, port, &mut inner) == Ok(()) {
                break;
            }
        }

        if inner_rc.borrow().slot_map.is_empty() {
            return Err(RedisError::Parse(
                "no seed node responded with cluster topology".into(),
            ));
        }

        Ok(Self { inner: inner_rc })
    }

    /// Connect to a seed node and query CLUSTER NODES.
    fn discover_seed(
        host: &str,
        port: u16,
        inner: &mut ClusterInner,
    ) -> Result<(), RedisError> {
        let conn = Connection::connect(host, port)
            .map_err(|e| RedisError::Parse(format!("seed connect failed: {e}")))?;

        let cmd = CommandBuilder::new("CLUSTER NODES")
            .build()
            .ok_or_else(|| {
                RedisError::Parse("CLUSTER NODES failed to encode".into())
            })?;
        let (tx, rx) = spsc::channel();
        let request = Request::new(cmd.to_vec(), tx);
        conn.send(request).map_err(|e| match e {
            ConnectionLimitError::QueueFull(n) => {
                RedisError::Parse(format!("request queue full: depth={n}"))
            }
            ConnectionLimitError::RequestTooLarge(max, got) => {
                RedisError::Parse(format!("request too large: {got}/{max}"))
            }
        })?;

        let response = rx
            .recv()
            .map_err(|_| RedisError::Parse("response channel closed".into()))?;

        if let RedisValue::BulkString(ref data) = response {
            let text = String::from_utf8_lossy(data);
            inner.slot_map = super::topology::parse_cluster_nodes(&text)?;
        } else {
            return Err(RedisError::Parse(format!(
                "unexpected CLUSTER NODES response type: {response:?}"
            )));
        }

        if inner.slot_map.is_complete() {
            if let Some(node_id) = inner.slot_map.nodes().next().map(|n| n.id) {
                inner.connections.insert(node_id, Arc::new(conn));
            }
        }
        Ok(())
    }

    /// Execute a command and return the typed result.
    ///
    /// Routes the command to the correct node based on the first key's
    /// hash slot: `CRC16(key) % 16384`.  If the response is a MOVED or ASK
    /// redirect the command is transparently retried on the correct node
    /// (up to 3 times).
    ///
    /// # Arguments
    /// * `cmd` — The command to execute, built with [`CommandBuilder`].
    ///
    /// # Returns
    /// The decoded response of type `T`, or a [`RedisError`] on failure.
    ///
    /// # Errors
    /// Returns [`RedisError::Connection`] if the connection fails,
    /// [`RedisError::Parse`] if the response cannot be decoded.
    #[allow(clippy::too_many_lines)]
    pub fn execute<T: FromRedisValue>(
        &self,
        cmd: CommandBuilder,
    ) -> Result<T, RedisError> {
        let key_bytes = extract_first_key(&cmd).ok_or_else(|| {
            RedisError::Parse("command requires at least one key".into())
        })?;
        let slot = compute_slot(&key_bytes);

        let encoded = cmd
            .build()
            .ok_or_else(|| RedisError::Parse("command encoding failed".into()))?;

        // Retry loop with redirect handling.
        let mut retries = 0u8;
        let mut current_slot = slot;
        let current_encoded = encoded;

        loop {
            // 1. Route to the correct node.
            let conn = match self.inner.borrow().connection_for_slot(current_slot) {
                Ok(c) => c,
                Err(e) => {
                    if matches!(
                        e,
                        RedisError::Parse(ref p) if p.contains("unknown slot")
                    ) {
                        self.refresh_topology()?;
                        self.inner
                            .borrow()
                            .connection_for_slot(current_slot)
                            .map_err(|_| {
                                RedisError::Parse(format!(
                                    "slot {current_slot} still unassigned after refresh"
                                ))
                            })?
                    } else {
                        return Err(e);
                    }
                }
            };

            // 2. Send the command.
            let (tx, rx) = spsc::channel();
            conn.send(Request::new(current_encoded.to_vec(), tx))
                .map_err(|e| match e {
                    ConnectionLimitError::QueueFull(n) => {
                        RedisError::Parse(format!("request queue full: depth={n}"))
                    }
                    ConnectionLimitError::RequestTooLarge(max, got) => {
                        RedisError::Parse(format!("request too large: {got}/{max}"))
                    }
                })?;

            // 3. Read the response.
            let response = rx.recv().map_err(|_| {
                RedisError::Parse("response channel closed — connection lost".into())
            })?;

            // 4. Check for redirects.
            if let Some(redirect) = super::redirect::parse_moved_redirect(&response) {
                retries += 1;
                if retries > 3 {
                    return Err(RedisError::Parse(
                        "max redirect attempts (3) exceeded".into(),
                    ));
                }
                // Update slot map with redirect target node.
                self.inner.borrow_mut().slot_map = {
                    let mut new_map = self.inner.borrow().slot_map.clone();
                    super::redirect::update_slot_map_on_redirect(
                        &mut new_map,
                        &redirect,
                    );
                    new_map
                };
                // Retry on the redirect target (same encoded command).
                current_slot = redirect.slot;
                continue;
            }
            if let Some(redirect) = super::redirect::parse_ask_redirect(&response) {
                retries += 1;
                if retries > 3 {
                    return Err(RedisError::Parse(
                        "max redirect attempts (3) exceeded".into(),
                    ));
                }
                // Update slot map with redirect target node.
                self.inner.borrow_mut().slot_map = {
                    let mut new_map = self.inner.borrow().slot_map.clone();
                    super::redirect::update_slot_map_on_redirect(
                        &mut new_map,
                        &redirect,
                    );
                    new_map
                };
                // Send ASKING to target node, then retry original command.
                let new_conn = {
                    self.inner.borrow_mut().slot_map = {
                        let mut new_map = self.inner.borrow().slot_map.clone();
                        super::redirect::update_slot_map_on_redirect(
                            &mut new_map,
                            &redirect,
                        );
                        new_map
                    };
                    self.inner
                        .borrow()
                        .connection_for_slot(redirect.slot)
                        .map_err(|_| {
                            RedisError::Parse(format!(
                                "no connection for node at {redirect}"
                            ))
                        })?
                };
                // Send ASKING.
                let asking =
                    CommandBuilder::new("ASKING").build().ok_or_else(|| {
                        RedisError::Parse("ASKING encoding failed".into())
                    })?;
                let (tx2, rx2) = spsc::channel();
                new_conn.send(Request::new(asking.to_vec(), tx2)).map_err(
                    |e| match e {
                        ConnectionLimitError::QueueFull(n) => {
                            RedisError::Parse(format!("request queue full: depth={n}"))
                        }
                        ConnectionLimitError::RequestTooLarge(max, got) => {
                            RedisError::Parse(format!("request too large: {got}/{max}"))
                        }
                    },
                )?;
                let _asking_resp = rx2.recv().map_err(|_| {
                    RedisError::Parse("ASKING response channel closed".into())
                })?;
                // Now send the original command on the new connection.
                let (tx3, rx3) = spsc::channel();
                new_conn
                    .send(Request::new(current_encoded.to_vec(), tx3))
                    .map_err(|e| match e {
                        ConnectionLimitError::QueueFull(n) => {
                            RedisError::Parse(format!("request queue full: depth={n}"))
                        }
                        ConnectionLimitError::RequestTooLarge(max, got) => {
                            RedisError::Parse(format!("request too large: {got}/{max}"))
                        }
                    })?;
                let response = rx3.recv().map_err(|_| {
                    RedisError::Parse("ASK retry response channel closed".into())
                })?;
                return T::from_redis_value(&response);
            }

            // Not a redirect — return the result.
            return T::from_redis_value(&response);
        }
    }

    /// Refresh the cluster topology from a live node.
    ///
    /// Queries `CLUSTER NODES` on the first available node and updates
    /// the slot map. Used for on-demand refresh after redirects or errors.
    ///
    /// # Errors
    /// Returns [`RedisError`] if no connections are available or parsing fails.
    ///
    /// # May runtime requirement
    /// Requires the may coroutine runtime.
    pub fn refresh_topology(&self) -> Result<(), RedisError> {
        let inner_arc = std::rc::Rc::clone(&self.inner);
        let conn = {
            let inner = inner_arc.borrow();
            inner
                .connections
                .values()
                .next()
                .ok_or_else(|| RedisError::Parse("no connections available".into()))?
                .clone()
        };

        let cmd = CommandBuilder::new("CLUSTER NODES")
            .build()
            .ok_or_else(|| RedisError::Parse("CLUSTER NODES encoding failed".into()))?;
        let (tx, rx) = spsc::channel();
        conn.send(Request::new(cmd.to_vec(), tx))
            .map_err(|e| match e {
                ConnectionLimitError::QueueFull(n) => {
                    RedisError::Parse(format!("request queue full: depth={n}"))
                }
                ConnectionLimitError::RequestTooLarge(max, got) => {
                    RedisError::Parse(format!("request too large: {got}/{max}"))
                }
            })?;

        let response = rx.recv().map_err(|_| {
            RedisError::Parse("response channel closed during topology refresh".into())
        })?;

        if let RedisValue::BulkString(ref data) = response {
            let text = String::from_utf8_lossy(data);
            let new_map = super::topology::parse_cluster_nodes(&text)?;
            inner_arc.borrow_mut().slot_map = new_map;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Key extraction
// ---------------------------------------------------------------------------

/// Extract the first key from a CommandBuilder for slot computation.
///
/// Redis Cluster uses the first key to determine the target slot.
/// We encode to RESP and parse the first argument byte-slice from the
/// encoded form since `CommandBuilder::args` is private.
///
/// Returns an owned `Vec<u8>` to avoid lifetime issues with the
/// temporary `BytesMut` produced by `build()`.
fn extract_first_key(cmd: &CommandBuilder) -> Option<Vec<u8>> {
    let encoded = cmd.clone().build()?;
    let bytes = encoded.as_ref();

    // Parse RESP array: *N\r\n$Len\r\nData\r\n$Len\r\nKey\r\n...
    // Skip array header: *N\r\n
    let mut i = 1usize;
    while i < bytes.len() && bytes[i] != b'\r' {
        i += 1;
    }
    i += 2; // skip \r\n

    // Skip command name: $Len\r\nData\r\n
    if i >= bytes.len() || bytes[i] != b'$' {
        return None;
    }
    i += 1;
    let mut len = 0u32;
    while i < bytes.len() && bytes[i] != b'\r' {
        len = len * 10 + u32::from(bytes[i] - b'0');
        i += 1;
    }
    i += 2; // skip \r\n
    if i + len as usize > bytes.len() {
        return None;
    }
    i += len as usize + 2; // skip data + \r\n

    // Now at first arg: $Len\r\nKey\r\n
    if i >= bytes.len() || bytes[i] != b'$' {
        return None;
    }
    i += 1;
    len = 0;
    while i < bytes.len() && bytes[i] != b'\r' {
        len = len * 10 + u32::from(bytes[i] - b'0');
        i += 1;
    }
    i += 2; // skip \r\n

    if i + len as usize > bytes.len() {
        return None;
    }
    Some(bytes[i..i + len as usize].to_vec())
}
