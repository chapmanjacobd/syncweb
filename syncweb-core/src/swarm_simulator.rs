//! Deterministic control-plane and replication simulation.
//!
//! The simulator deliberately models policy rather than sockets or blob bytes.
//! It is useful for exercising inventory propagation, provider loss,
//! partitions, CAR imports, and repair decisions without requiring a live LAN.

use std::collections::{BTreeMap, BTreeSet};

use iroh_blobs::Hash;

use crate::{Result, SyncwebError};

/// A stable identifier for one simulated node.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub struct SimNodeId(pub u32);

impl SimNodeId {
    /// Construct a simulated node identifier.
    #[must_use]
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Return the numeric node identifier.
    #[must_use]
    pub const fn value(self) -> u32 {
        self.0
    }
}

/// Faults and resource failures injected into a simulation.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct FaultConfig {
    /// Percentage of inventory or repair messages that are dropped.
    pub packet_loss_percent: u8,
    /// Number of rounds for which an inventory observation remains usable.
    pub lease_ttl_rounds: u64,
    /// Nodes that can only use a relay path. In the model this permits
    /// communication across partitions when either endpoint is listed.
    pub relay_only_nodes: BTreeSet<SimNodeId>,
    /// Nodes that reject all new content, simulating an exhausted disk.
    pub disk_exhausted_nodes: BTreeSet<SimNodeId>,
    /// Per-node clock offsets used when issuing provider observations.
    pub clock_skew_rounds: BTreeMap<SimNodeId, i64>,
    /// Nodes to shut down when the simulation reaches the given round.
    pub abrupt_shutdown_at: BTreeMap<SimNodeId, u64>,
}

impl Default for FaultConfig {
    fn default() -> Self {
        Self {
            packet_loss_percent: 0,
            lease_ttl_rounds: 3,
            relay_only_nodes: BTreeSet::new(),
            disk_exhausted_nodes: BTreeSet::new(),
            clock_skew_rounds: BTreeMap::new(),
            abrupt_shutdown_at: BTreeMap::new(),
        }
    }
}

impl FaultConfig {
    /// Set packet loss, rejecting percentages outside the inclusive range.
    ///
    /// # Errors
    ///
    /// Returns an error when `percent` is greater than 100.
    pub fn set_packet_loss(&mut self, percent: u8) -> Result<()> {
        if percent > 100 {
            return Err(SyncwebError::InvalidConfig(
                "simulation packet loss must be between 0 and 100".to_owned(),
            ));
        }
        self.packet_loss_percent = percent;
        Ok(())
    }

    /// Configure the lease lifetime in simulation rounds.
    #[must_use]
    pub const fn with_lease_ttl(mut self, rounds: u64) -> Self {
        self.lease_ttl_rounds = rounds;
        self
    }
}

/// Configuration for a deterministic swarm simulation.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct SimulationConfig {
    /// Minimum number of online replicas required for each known block.
    pub replica_target: usize,
    /// Seed for deterministic message-loss decisions.
    pub seed: u64,
    /// Faults applied while the simulation runs.
    pub faults: FaultConfig,
}

impl Default for SimulationConfig {
    fn default() -> Self {
        Self {
            replica_target: 2,
            seed: 0x5eed,
            faults: FaultConfig::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Observation {
    observed_at: u64,
    expires_at: u64,
}

#[derive(Clone, Debug, Default)]
struct SimNode {
    online: bool,
    partition: u32,
    inventory: BTreeSet<Hash>,
    observations: BTreeMap<(SimNodeId, Hash), Observation>,
}

/// Summary of one completed simulation.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub struct SimulationReport {
    /// Number of rounds executed.
    pub rounds: u64,
    /// Number of blocks known by at least one node.
    pub known_blocks: usize,
    /// Number of successful content repairs/transfers.
    pub repairs: u64,
    /// Number of repair attempts made for missing replicas.
    pub repair_attempts: u64,
    /// Number of dropped control or repair messages.
    pub dropped_messages: u64,
    /// Number of repair attempts rejected by a fault or unavailable provider.
    pub failed_repairs: u64,
    /// Number of inventory advertisements sent to reachable peers.
    pub inventory_messages: u64,
    /// Approximate inventory bytes sent, counting one 32-byte hash per item.
    pub inventory_bytes: u64,
    /// Number of inventory advertisements repeating an existing observation.
    pub duplicate_inventory_messages: u64,
    /// Whether every known block reached the configured replica target.
    pub converged: bool,
}

/// A deterministic, bounded simulation of provider discovery and repair.
#[derive(Clone, Debug)]
pub struct SwarmSimulator {
    config: SimulationConfig,
    nodes: BTreeMap<SimNodeId, SimNode>,
    round: u64,
    random_state: u64,
    repairs: u64,
    repair_attempts: u64,
    dropped_messages: u64,
    failed_repairs: u64,
    inventory_messages: u64,
    inventory_bytes: u64,
    duplicate_inventory_messages: u64,
}

impl SwarmSimulator {
    /// Create `node_count` nodes in one connected partition.
    ///
    /// # Errors
    ///
    /// Returns an error when no nodes are requested, the replica target is
    /// zero, or packet loss is outside the valid range.
    pub fn new(node_count: u32, config: SimulationConfig) -> Result<Self> {
        if node_count == 0 {
            return Err(SyncwebError::InvalidConfig(
                "swarm simulation requires at least one node".to_owned(),
            ));
        }
        if config.replica_target == 0 {
            return Err(SyncwebError::InvalidConfig(
                "swarm simulation replica target must be non-zero".to_owned(),
            ));
        }
        if config.faults.packet_loss_percent > 100 {
            return Err(SyncwebError::InvalidConfig(
                "simulation packet loss must be between 0 and 100".to_owned(),
            ));
        }
        let nodes = (0..node_count)
            .map(|id| {
                (
                    SimNodeId(id),
                    SimNode {
                        online: true,
                        ..SimNode::default()
                    },
                )
            })
            .collect();
        let random_state = config.seed;
        Ok(Self {
            config,
            nodes,
            round: 0,
            random_state,
            repairs: 0,
            repair_attempts: 0,
            dropped_messages: 0,
            failed_repairs: 0,
            inventory_messages: 0,
            inventory_bytes: 0,
            duplicate_inventory_messages: 0,
        })
    }

    /// Import one verified content identifier into a node.
    ///
    /// # Errors
    ///
    /// Returns an error when the node does not exist, is offline, or is
    /// configured as disk-exhausted.
    pub fn import_block(&mut self, node: SimNodeId, hash: Hash) -> Result<()> {
        self.ensure_node(node)?;
        if self.config.faults.disk_exhausted_nodes.contains(&node) {
            return Err(SyncwebError::operation(
                "simulation import rejected",
                "node disk is exhausted",
            ));
        }
        let state = self.node_mut(node)?;
        if !state.online {
            return Err(SyncwebError::operation("simulation import rejected", "node is offline"));
        }
        state.inventory.insert(hash);
        Ok(())
    }

    /// Import a verified CAR block set atomically.
    ///
    /// The simulator treats the identifiers as already hash-verified by the
    /// archive reader; if the target cannot accept the complete set, no block
    /// is added.
    ///
    /// # Errors
    ///
    /// Returns an error when the target cannot accept the import.
    pub fn import_archive<I>(&mut self, node: SimNodeId, block_iter: I) -> Result<usize>
    where
        I: IntoIterator<Item = Hash>,
    {
        self.ensure_node(node)?;
        let blocks = block_iter.into_iter().collect::<BTreeSet<_>>();
        let state = self.node(node)?;
        if !state.online {
            return Err(SyncwebError::operation(
                "simulation archive import rejected",
                "node is offline",
            ));
        }
        if self.config.faults.disk_exhausted_nodes.contains(&node) {
            return Err(SyncwebError::operation(
                "simulation archive import rejected",
                "node disk is exhausted",
            ));
        }
        let new_blocks = blocks.difference(&state.inventory).count();
        self.node_mut(node)?.inventory.extend(blocks);
        Ok(new_blocks)
    }

    /// Move a node into a partition.
    ///
    /// Nodes in different partitions cannot communicate directly. A relay-only
    /// endpoint remains eligible for cross-partition communication.
    ///
    /// # Errors
    ///
    /// Returns an error when the node does not exist.
    pub fn set_partition(&mut self, node: SimNodeId, partition: u32) -> Result<()> {
        self.node_mut(node)?.partition = partition;
        Ok(())
    }

    /// Set whether a node is available for control and data traffic.
    ///
    /// Taking a node offline clears its provider observations, modelling a
    /// provider failure rather than merely hiding it from selection.
    ///
    /// # Errors
    ///
    /// Returns an error when the node does not exist.
    pub fn set_online(&mut self, node: SimNodeId, online: bool) -> Result<()> {
        let state = self.node_mut(node)?;
        state.online = online;
        if !online {
            state.observations.clear();
        }
        Ok(())
    }

    /// Execute one inventory-and-repair round.
    pub fn step(&mut self) {
        self.apply_scheduled_shutdowns();
        self.expire_observations();
        self.propagate_inventory();
        self.repair_replication();
        self.round = self.round.saturating_add(1);
    }

    /// Run until the configured replica target is met or `max_rounds` elapse.
    #[must_use]
    pub fn run_until_converged(&mut self, max_rounds: u64) -> SimulationReport {
        let mut rounds = 0_u64;
        while rounds < max_rounds && !self.is_converged() {
            self.step();
            rounds = rounds.saturating_add(1);
        }
        self.report(rounds)
    }

    /// Return the current simulation round.
    #[must_use]
    pub const fn round(&self) -> u64 {
        self.round
    }

    /// Return the current blocks stored by a node.
    ///
    /// # Errors
    ///
    /// Returns an error when the node does not exist.
    pub fn node_inventory(&self, node: SimNodeId) -> Result<Vec<Hash>> {
        Ok(self.node(node)?.inventory.iter().copied().collect())
    }

    /// Return fresh provider observations for a block at a node.
    ///
    /// # Errors
    ///
    /// Returns an error when the node does not exist.
    pub fn active_provider_count(&self, node: SimNodeId, hash: Hash) -> Result<usize> {
        let state = self.node(node)?;
        Ok(state
            .observations
            .iter()
            .filter(|((provider, observed_hash), observation)| {
                if *observed_hash != hash {
                    return false;
                }
                if observation.expires_at <= self.round {
                    return false;
                }
                self.nodes
                    .get(provider)
                    .is_some_and(|provider_state| provider_state.online)
            })
            .count())
    }

    /// Return a report without advancing the simulation.
    #[must_use]
    pub fn report(&self, rounds: u64) -> SimulationReport {
        SimulationReport {
            rounds,
            known_blocks: self.known_blocks().len(),
            repairs: self.repairs,
            repair_attempts: self.repair_attempts,
            dropped_messages: self.dropped_messages,
            failed_repairs: self.failed_repairs,
            inventory_messages: self.inventory_messages,
            inventory_bytes: self.inventory_bytes,
            duplicate_inventory_messages: self.duplicate_inventory_messages,
            converged: self.is_converged(),
        }
    }

    fn ensure_node(&self, node: SimNodeId) -> Result<()> {
        if self.nodes.contains_key(&node) {
            Ok(())
        } else {
            Err(SyncwebError::InvalidConfig(format!(
                "simulation node {} does not exist",
                node.0
            )))
        }
    }

    fn node(&self, node: SimNodeId) -> Result<&SimNode> {
        self.nodes
            .get(&node)
            .ok_or_else(|| SyncwebError::InvalidConfig(format!("simulation node {} does not exist", node.0)))
    }

    fn node_mut(&mut self, node: SimNodeId) -> Result<&mut SimNode> {
        self.nodes
            .get_mut(&node)
            .ok_or_else(|| SyncwebError::InvalidConfig(format!("simulation node {} does not exist", node.0)))
    }

    fn apply_scheduled_shutdowns(&mut self) {
        let nodes = self
            .config
            .faults
            .abrupt_shutdown_at
            .iter()
            .filter_map(|(node, shutdown_round)| (*shutdown_round == self.round).then_some(*node))
            .collect::<Vec<_>>();
        for node in nodes {
            if let Some(state) = self.nodes.get_mut(&node) {
                state.online = false;
                state.observations.clear();
            }
        }
    }

    fn expire_observations(&mut self) {
        for state in self.nodes.values_mut() {
            state
                .observations
                .retain(|_, observation| observation.expires_at > self.round);
        }
    }

    fn propagate_inventory(&mut self) {
        let online_nodes = self
            .nodes
            .iter()
            .filter_map(|(node, state)| state.online.then_some(*node))
            .collect::<Vec<_>>();
        for source in &online_nodes {
            let hashes = self
                .nodes
                .get(source)
                .map(|state| state.inventory.iter().copied().collect::<Vec<_>>())
                .unwrap_or_default();
            for target in &online_nodes {
                if source == target || !self.can_exchange(*source, *target) {
                    continue;
                }
                for hash in &hashes {
                    self.inventory_messages = self.inventory_messages.saturating_add(1);
                    self.inventory_bytes = self.inventory_bytes.saturating_add(32);
                    if self
                        .nodes
                        .get(target)
                        .is_some_and(|state| state.observations.contains_key(&(*source, *hash)))
                    {
                        self.duplicate_inventory_messages = self.duplicate_inventory_messages.saturating_add(1);
                    }
                    if self.message_is_dropped() {
                        self.dropped_messages = self.dropped_messages.saturating_add(1);
                        continue;
                    }
                    let source_clock = self
                        .config
                        .faults
                        .clock_skew_rounds
                        .get(source)
                        .copied()
                        .unwrap_or_default();
                    let observed_at = shifted_round(self.round, source_clock);
                    let expires_at = observed_at.saturating_add(self.config.faults.lease_ttl_rounds);
                    if let Some(state) = self.nodes.get_mut(target) {
                        state.observations.insert(
                            (*source, *hash),
                            Observation {
                                observed_at,
                                expires_at,
                            },
                        );
                    }
                }
            }
        }
    }

    fn repair_replication(&mut self) {
        let hashes = self.known_blocks();
        for hash in hashes {
            let replicas = self
                .nodes
                .iter()
                .filter(|(_, state)| state.online && state.inventory.contains(&hash))
                .map(|(node, _)| *node)
                .collect::<Vec<_>>();
            if replicas.len() >= self.config.replica_target {
                continue;
            }
            let targets = self
                .nodes
                .iter()
                .filter_map(|(node, state)| (state.online && !state.inventory.contains(&hash)).then_some(*node))
                .collect::<Vec<_>>();
            for target in targets {
                if self
                    .nodes
                    .get(&target)
                    .is_some_and(|state| state.inventory.contains(&hash))
                {
                    continue;
                }
                self.repair_attempts = self.repair_attempts.saturating_add(1);
                let has_provider = replicas.iter().copied().any(|source| {
                    self.can_exchange(source, target)
                        && self
                            .nodes
                            .get(&target)
                            .is_some_and(|state| state.observations.contains_key(&(source, hash)))
                });
                if !has_provider {
                    self.failed_repairs = self.failed_repairs.saturating_add(1);
                    continue;
                }
                if self.config.faults.disk_exhausted_nodes.contains(&target) {
                    self.failed_repairs = self.failed_repairs.saturating_add(1);
                    continue;
                }
                if self.message_is_dropped() {
                    self.dropped_messages = self.dropped_messages.saturating_add(1);
                    self.failed_repairs = self.failed_repairs.saturating_add(1);
                    continue;
                }
                if let Some(state) = self.nodes.get_mut(&target) {
                    state.inventory.insert(hash);
                    self.repairs = self.repairs.saturating_add(1);
                }
                if self
                    .nodes
                    .values()
                    .filter(|state| state.online && state.inventory.contains(&hash))
                    .count()
                    >= self.config.replica_target
                {
                    break;
                }
            }
        }
    }

    fn known_blocks(&self) -> BTreeSet<Hash> {
        self.nodes
            .values()
            .flat_map(|state| state.inventory.iter().copied())
            .collect()
    }

    fn is_converged(&self) -> bool {
        let hashes = self.known_blocks();
        !hashes.is_empty()
            && hashes.iter().all(|hash| {
                self.nodes
                    .values()
                    .filter(|state| state.online && state.inventory.contains(hash))
                    .count()
                    >= self.config.replica_target
            })
    }

    fn can_exchange(&self, source: SimNodeId, target: SimNodeId) -> bool {
        let Some(source_state) = self.nodes.get(&source) else {
            return false;
        };
        let Some(target_state) = self.nodes.get(&target) else {
            return false;
        };
        source_state.partition == target_state.partition
            || self.config.faults.relay_only_nodes.contains(&source)
            || self.config.faults.relay_only_nodes.contains(&target)
    }

    fn message_is_dropped(&mut self) -> bool {
        if self.config.faults.packet_loss_percent == 0 {
            return false;
        }
        let value = next_random(&mut self.random_state) % 100;
        value < u64::from(self.config.faults.packet_loss_percent)
    }
}

const fn shifted_round(round: u64, skew: i64) -> u64 {
    if skew.is_negative() {
        round.saturating_sub(skew.unsigned_abs())
    } else {
        round.saturating_add(skew.cast_unsigned())
    }
}

const fn next_random(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    *state
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hash(value: u8) -> Hash {
        Hash::from_bytes([value; 32])
    }

    #[test]
    fn replication_is_deterministic() {
        let config = SimulationConfig::default();
        let mut first = SwarmSimulator::new(4, config.clone()).expect("simulator should start");
        let mut second = SwarmSimulator::new(4, config).expect("simulator should start");
        first.import_block(SimNodeId(0), hash(1)).expect("import should work");
        second.import_block(SimNodeId(0), hash(1)).expect("import should work");

        let first_report = first.run_until_converged(10);
        let second_report = second.run_until_converged(10);

        assert_eq!(first_report, second_report);
        assert_eq!(first.node_inventory(SimNodeId(1)).expect("node exists"), vec![hash(1)]);
    }

    #[test]
    fn partition_rejoin_repairs_after_car_import() {
        let mut simulator = SwarmSimulator::new(
            4,
            SimulationConfig {
                replica_target: 4,
                ..SimulationConfig::default()
            },
        )
        .expect("simulator should start");
        simulator.set_partition(SimNodeId(2), 1).expect("node exists");
        simulator.set_partition(SimNodeId(3), 1).expect("node exists");
        assert_eq!(
            simulator
                .import_archive(SimNodeId(0), [hash(1), hash(2), hash(2)])
                .expect("archive import should work"),
            2
        );
        simulator.step();
        assert!(simulator.node_inventory(SimNodeId(2)).expect("node exists").is_empty());

        simulator.set_partition(SimNodeId(2), 0).expect("node exists");
        simulator.set_partition(SimNodeId(3), 0).expect("node exists");
        let report = simulator.run_until_converged(10);

        assert!(report.converged);
        assert_eq!(report.known_blocks, 2);
        assert_eq!(simulator.node_inventory(SimNodeId(3)).expect("node exists").len(), 2);
    }

    #[test]
    fn injected_faults_are_observable_and_recoverable() {
        let mut faults = FaultConfig::default().with_lease_ttl(2);
        faults.set_packet_loss(100).expect("valid packet loss");
        faults.relay_only_nodes.insert(SimNodeId(0));
        faults.disk_exhausted_nodes.insert(SimNodeId(2));
        faults.clock_skew_rounds.insert(SimNodeId(0), 5);
        faults.abrupt_shutdown_at.insert(SimNodeId(1), 1);
        let mut simulator = SwarmSimulator::new(
            3,
            SimulationConfig {
                replica_target: 2,
                faults,
                ..SimulationConfig::default()
            },
        )
        .expect("simulator should start");
        simulator.set_partition(SimNodeId(2), 1).expect("node exists");
        simulator
            .import_block(SimNodeId(0), hash(9))
            .expect("import should work");
        let first = simulator.run_until_converged(1);
        assert!(!first.converged);
        assert!(first.dropped_messages > 0);
        assert!(first.failed_repairs > 0);

        simulator.config.faults.packet_loss_percent = 0;
        simulator.config.faults.disk_exhausted_nodes.clear();
        simulator.set_online(SimNodeId(1), true).expect("node exists");
        let recovered = simulator.run_until_converged(10);
        assert!(recovered.converged);
        assert!(
            simulator
                .active_provider_count(SimNodeId(2), hash(9))
                .expect("node exists")
                > 0
        );
    }

    #[test]
    fn stale_observations_expire() {
        let mut simulator = SwarmSimulator::new(
            2,
            SimulationConfig {
                replica_target: 2,
                faults: FaultConfig::default().with_lease_ttl(2),
                ..SimulationConfig::default()
            },
        )
        .expect("simulator should start");
        simulator
            .import_block(SimNodeId(0), hash(4))
            .expect("import should work");
        simulator.step();
        assert_eq!(
            simulator
                .active_provider_count(SimNodeId(1), hash(4))
                .expect("node exists"),
            1
        );
        simulator.set_online(SimNodeId(0), false).expect("node exists");
        simulator.step();
        assert_eq!(
            simulator
                .active_provider_count(SimNodeId(1), hash(4))
                .expect("node exists"),
            0
        );
    }

    #[test]
    fn scale_fixture_covers_lan_gate_sizes() {
        for node_count in [10, 50, 100, 1_000] {
            let mut simulator = SwarmSimulator::new(
                node_count,
                SimulationConfig {
                    replica_target: 2,
                    seed: 0x5eed,
                    ..SimulationConfig::default()
                },
            )
            .expect("scale fixture should create a simulator");
            simulator
                .import_block(SimNodeId::new(0), hash(11))
                .expect("scale fixture import should work");

            let report = simulator.run_until_converged(4);
            assert!(
                report.converged,
                "simulation should converge for {node_count} nodes: {report:?}"
            );
            assert_eq!(report.known_blocks, 1);
            assert_eq!(
                simulator
                    .node_inventory(SimNodeId::new(1))
                    .expect("scale fixture node should exist"),
                vec![hash(11)]
            );
            assert!(report.inventory_messages > 0);
            assert!(report.inventory_bytes > 0);
            assert!(report.repair_attempts > 0);
            assert!(report.repairs > 0);
            println!(
                "scale nodes={node_count} rounds={} inventory_messages={} inventory_bytes={} \
                 duplicate_inventory_messages={} repair_attempts={} completed_repairs={} \
                 failed_repairs={} open_fds={:?}",
                report.rounds,
                report.inventory_messages,
                report.inventory_bytes,
                report.duplicate_inventory_messages,
                report.repair_attempts,
                report.repairs,
                report.failed_repairs,
                std::fs::read_dir("/proc/self/fd").ok().map(|entries| entries.count()),
            );
        }
    }
}
