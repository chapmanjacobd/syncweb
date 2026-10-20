//! Scope-aware replica targets and deterministic repair planning.
//!
//! The controller is deliberately transport-agnostic. Callers provide the
//! current provider observations and confirmed replicas, then run reconciliation
//! again when leases expire, requests fail, or membership changes.

use std::collections::{HashMap, HashSet};

use iroh::PublicKey;
use iroh_blobs::Hash;
use serde::{Deserialize, Serialize};

use crate::{Result, SyncwebError};

/// The policy scope to which a replica target applies.
///
/// `None` is a wildcard at that scope. More-specific scopes override
/// less-specific scopes before priority is considered.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ReplicaScope {
    pub community: Option<String>,
    pub folder: Option<String>,
    pub collection: Option<String>,
}

impl ReplicaScope {
    /// Construct an unscoped target that applies to every content item.
    #[must_use]
    pub const fn global() -> Self {
        Self {
            community: None,
            folder: None,
            collection: None,
        }
    }

    /// Return whether this scope applies to the requested content scope.
    #[must_use]
    pub fn matches(&self, requested: &Self) -> bool {
        self.community
            .as_deref()
            .is_none_or(|value| requested.community.as_deref() == Some(value))
            && self
                .folder
                .as_deref()
                .is_none_or(|value| requested.folder.as_deref() == Some(value))
            && self
                .collection
                .as_deref()
                .is_none_or(|value| requested.collection.as_deref() == Some(value))
    }

    fn specificity(&self) -> usize {
        usize::from(self.community.is_some())
            .saturating_add(usize::from(self.folder.is_some()))
            .saturating_add(usize::from(self.collection.is_some()))
    }
}

/// Desired replica bounds for one scope.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct ReplicaTarget {
    pub scope: ReplicaScope,
    pub min_replicas: usize,
    pub max_replicas: Option<usize>,
    pub priority: u8,
}

impl ReplicaTarget {
    /// Create a target with an optional upper bound.
    ///
    /// # Errors
    ///
    /// Returns an error when the maximum is lower than the minimum.
    pub fn new(scope: ReplicaScope, min_replicas: usize, max_replicas: Option<usize>, priority: u8) -> Result<Self> {
        if max_replicas.is_some_and(|max| max < min_replicas) {
            return Err(SyncwebError::InvalidConfig(
                "replica target maximum must not be lower than its minimum".to_owned(),
            ));
        }
        Ok(Self {
            scope,
            min_replicas,
            max_replicas,
            priority,
        })
    }

    /// Return the upper bound, or an unbounded value when none is configured.
    #[must_use]
    pub const fn upper_bound(&self) -> usize {
        match self.max_replicas {
            Some(max) => max,
            None => usize::MAX,
        }
    }
}

/// The transport path observed for a provider.
///
/// This is an advisory ordering hint only. Lease validation, availability, and
/// capacity checks remain authoritative for provider eligibility.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub enum ProviderPath {
    Direct,
    Relay,
    #[default]
    Unknown,
}

/// Advisory topology information used to order eligible providers.
///
/// Interface and subnet values are opaque labels supplied by discovery. They
/// are never used for authorization or community membership decisions.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub struct ProviderTopology {
    pub interface: Option<String>,
    pub subnet: Option<String>,
    pub latency_ms: Option<u32>,
    pub path: ProviderPath,
}

impl ProviderTopology {
    /// Return a deterministic preference score for provider selection.
    #[must_use]
    pub fn preference_score(&self) -> u64 {
        let path_score = match self.path {
            ProviderPath::Direct => 3_u64,
            ProviderPath::Relay => 2,
            ProviderPath::Unknown => 1,
        };
        let locality_score = u64::from(self.interface.is_some()).saturating_add(u64::from(self.subnet.is_some()));
        let latency_score = self.latency_ms.map_or(0, |latency| {
            1_000_000_u64.saturating_sub(u64::from(latency).min(1_000_000))
        });
        path_score
            .saturating_mul(2_000_000)
            .saturating_add(locality_score.saturating_mul(1_000_000))
            .saturating_add(latency_score)
    }
}

/// A provider currently eligible to receive a repair request.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct ProviderState {
    pub provider: PublicKey,
    pub lease_expires_at: u64,
    pub available: bool,
    pub capacity_available: bool,
    pub score: u64,
    pub topology: ProviderTopology,
}

impl ProviderState {
    /// Return whether this provider may be selected at `now`.
    #[must_use]
    pub const fn eligible_at(&self, now: u64) -> bool {
        self.available && self.capacity_available && now < self.lease_expires_at
    }
}

/// A bounded reservation for a provider to receive one content replica.
///
/// Active reservations count toward the configured maximum so repeated repair
/// passes do not issue duplicate requests. They count toward the minimum only
/// after the provider confirms both the pin and the reserved capacity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct ProviderReservation {
    pub hash: Hash,
    pub provider: PublicKey,
    pub reserved_at: u64,
    pub expires_at: u64,
    pub confirmed: bool,
    pub capacity_reserved: bool,
}

impl ProviderReservation {
    /// Create a reservation with a finite lifetime.
    ///
    /// # Errors
    ///
    /// Returns an error when the reservation would expire before it is made.
    pub fn new(
        hash: Hash,
        provider: PublicKey,
        reserved_at: u64,
        expires_at: u64,
        confirmed: bool,
        capacity_reserved: bool,
    ) -> Result<Self> {
        if expires_at <= reserved_at {
            return Err(SyncwebError::InvalidConfig(
                "provider reservation expiry must be after reservation time".to_owned(),
            ));
        }
        Ok(Self {
            hash,
            provider,
            reserved_at,
            expires_at,
            confirmed,
            capacity_reserved,
        })
    }

    /// Return whether this reservation is still usable at `now`.
    #[must_use]
    pub const fn is_active(self, now: u64) -> bool {
        self.reserved_at <= now && now < self.expires_at && self.capacity_reserved
    }

    /// Return whether this reservation satisfies the confirmed replica count.
    #[must_use]
    pub const fn is_confirmed_at(self, now: u64) -> bool {
        self.is_active(now) && self.confirmed
    }
}

/// Current replica observations for one content hash.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct ReplicaContent {
    pub hash: Hash,
    pub known_replicas: Vec<PublicKey>,
    pub confirmed_replicas: Vec<PublicKey>,
    /// Earliest timestamp at which a retry may be scheduled after a failure.
    pub repair_after: Option<u64>,
}

impl ReplicaContent {
    /// Construct observations for one content hash.
    #[must_use]
    pub const fn new(hash: Hash, known_replicas: Vec<PublicKey>, confirmed_replicas: Vec<PublicKey>) -> Self {
        Self {
            hash,
            known_replicas,
            confirmed_replicas,
            repair_after: None,
        }
    }

    /// Defer repair until `timestamp`, allowing callers to apply a retry
    /// delay returned by [`RepairPolicy::retry_at`].
    #[must_use]
    pub const fn with_repair_after(mut self, timestamp: u64) -> Self {
        self.repair_after = Some(timestamp);
        self
    }
}

/// One provider selected to repair a content hash.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct ReplicationRequest {
    pub hash: Hash,
    pub provider: PublicKey,
}

/// Replica counts and repair actions for one content hash.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct ReplicaStatus {
    pub hash: Hash,
    pub known_replicas: usize,
    pub confirmed_replicas: usize,
    pub requests: Vec<ReplicationRequest>,
}

/// Result of one reconciliation pass.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct ReplicationPlan {
    pub target: ReplicaTarget,
    pub statuses: Vec<ReplicaStatus>,
}

impl ReplicationPlan {
    /// Return all repair requests in content order.
    #[must_use]
    pub fn requests(&self) -> Vec<ReplicationRequest> {
        self.statuses
            .iter()
            .flat_map(|status| status.requests.iter().copied())
            .collect()
    }
}

/// Deterministic repair spreading policy.
///
/// The policy does not use wall-clock randomness. Every node derives the same
/// retry delay from the content hash, while ownership buckets assign a
/// content item to a stable subset of repair coordinators.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct RepairPolicy {
    /// Maximum additional delay applied to a failed repair attempt.
    pub jitter_window: u64,
    /// Number of deterministic ownership buckets. `0` behaves as `1`.
    pub ownership_buckets: u16,
}

impl Default for RepairPolicy {
    fn default() -> Self {
        Self {
            jitter_window: 0,
            ownership_buckets: 1,
        }
    }
}

impl RepairPolicy {
    /// Return a deterministic retry timestamp derived from `failed_at`.
    #[must_use]
    pub fn retry_at(self, hash: Hash, failed_at: u64) -> u64 {
        self.retry_at_seed(prefix_u64(hash.as_bytes()), failed_at)
    }

    /// Return an owner-specific retry timestamp so repair coordinators do not
    /// wake at the same instant for the same content.
    #[must_use]
    pub fn retry_at_for(self, hash: Hash, owner: PublicKey, failed_at: u64) -> u64 {
        let hash_seed = prefix_u64(hash.as_bytes());
        let owner_seed = prefix_u64(owner.as_bytes());
        self.retry_at_seed(hash_seed ^ owner_seed, failed_at)
    }

    const fn retry_at_seed(self, seed: u64, failed_at: u64) -> u64 {
        let span = self.jitter_window.saturating_add(1);
        let offset = match seed.checked_rem(span) {
            Some(value) => value,
            None => 0,
        };
        failed_at.saturating_add(offset)
    }

    /// Return whether `owner` is assigned to repair `hash`.
    #[must_use]
    pub fn owns(self, hash: Hash, owner: PublicKey) -> bool {
        let buckets = u32::from(self.ownership_buckets.max(1));
        let hash_bucket = prefix_u32(hash.as_bytes()).checked_rem(buckets).unwrap_or(0);
        let owner_bucket = prefix_u32(owner.as_bytes()).checked_rem(buckets).unwrap_or(0);
        hash_bucket == owner_bucket
    }
}

fn prefix_u64(bytes: &[u8]) -> u64 {
    bytes.first_chunk::<8>().map_or(0, |prefix| u64::from_le_bytes(*prefix))
}

fn prefix_u32(bytes: &[u8]) -> u32 {
    bytes.first_chunk::<4>().map_or(0, |prefix| u32::from_le_bytes(*prefix))
}

/// Stateless replication controller.
///
/// A fresh plan is computed from every call's observations. This makes
/// expiration, failed requests, and membership changes visible without
/// retaining stale provider state in the controller.
#[derive(Clone, Debug, Default)]
pub struct ReplicationController {
    targets: Vec<ReplicaTarget>,
    repair_policy: RepairPolicy,
    repair_owner: Option<PublicKey>,
}

impl ReplicationController {
    /// Create a controller from validated replica targets.
    #[must_use]
    pub const fn new(targets: Vec<ReplicaTarget>) -> Self {
        Self {
            targets,
            repair_policy: RepairPolicy {
                jitter_window: 0,
                ownership_buckets: 1,
            },
            repair_owner: None,
        }
    }

    /// Add a target and return the updated controller.
    #[must_use]
    pub fn with_target(mut self, target: ReplicaTarget) -> Self {
        self.targets.push(target);
        self
    }

    /// Configure deterministic repair spreading for this controller.
    #[must_use]
    pub const fn with_repair_policy(mut self, policy: RepairPolicy, owner: Option<PublicKey>) -> Self {
        self.repair_policy = policy;
        self.repair_owner = owner;
        self
    }

    /// Return the next retry timestamp for a failed content repair.
    #[must_use]
    pub fn repair_after(&self, hash: Hash, failed_at: u64) -> u64 {
        self.repair_owner.map_or_else(
            || self.repair_policy.retry_at(hash, failed_at),
            |owner| self.repair_policy.retry_at_for(hash, owner, failed_at),
        )
    }

    /// Select the most-specific matching target, breaking ties by priority.
    #[must_use]
    pub fn target_for(&self, scope: &ReplicaScope) -> Option<&ReplicaTarget> {
        let mut selected = None;
        for target in &self.targets {
            if !target.scope.matches(scope) {
                continue;
            }
            let should_replace = selected.is_none_or(|current: &ReplicaTarget| {
                target.scope.specificity() > current.scope.specificity()
                    || (target.scope.specificity() == current.scope.specificity() && target.priority > current.priority)
            });
            if should_replace {
                selected = Some(target);
            }
        }
        selected
    }

    /// Reconcile current content and provider observations into repair work.
    ///
    /// Known providers count toward the configured maximum, while only
    /// confirmed providers count toward the minimum. Providers must have a
    /// fresh lease, available capacity, and an available path before they are
    /// selected. Selection is deterministic by score and provider identity.
    ///
    /// # Errors
    ///
    /// Returns an error when no target matches `scope`.
    pub fn reconcile(
        &self,
        scope: &ReplicaScope,
        now: u64,
        contents: &[ReplicaContent],
        providers: &[ProviderState],
    ) -> Result<ReplicationPlan> {
        self.reconcile_with_reservations(scope, now, contents, providers, &[])
    }

    /// Reconcile content while accounting for active provider reservations.
    ///
    /// Expired reservations are ignored. Active unconfirmed reservations
    /// occupy capacity and prevent duplicate requests, but do not satisfy the
    /// minimum replica target until `confirmed` is true.
    ///
    /// # Errors
    ///
    /// Returns an error when no target matches `scope`.
    pub fn reconcile_with_reservations(
        &self,
        scope: &ReplicaScope,
        now: u64,
        contents: &[ReplicaContent],
        providers: &[ProviderState],
        reservations: &[ProviderReservation],
    ) -> Result<ReplicationPlan> {
        let target = self
            .target_for(scope)
            .cloned()
            .ok_or_else(|| SyncwebError::InvalidConfig("no replica target matches the content scope".to_owned()))?;
        let provider_states: HashMap<PublicKey, ProviderState> = providers
            .iter()
            .cloned()
            .map(|provider| (provider.provider, provider))
            .collect();

        let mut statuses = Vec::with_capacity(contents.len());
        for content in contents {
            let mut known = content.known_replicas.iter().copied().collect::<HashSet<_>>();
            let mut confirmed = content
                .confirmed_replicas
                .iter()
                .copied()
                .filter(|provider| known.contains(provider))
                .collect::<HashSet<_>>();
            // An explicitly expired or unavailable provider no longer counts
            // toward the target. Unknown observations remain counted until a
            // newer lease or failure report says otherwise.
            known.retain(|provider| {
                provider_states
                    .get(provider)
                    .is_none_or(|state| state.available && now < state.lease_expires_at)
            });
            confirmed.retain(|provider| known.contains(provider));
            for reservation in reservations
                .iter()
                .copied()
                .filter(|reservation| reservation.hash == content.hash && reservation.is_active(now))
            {
                known.insert(reservation.provider);
            }
            for reservation in reservations
                .iter()
                .copied()
                .filter(|reservation| reservation.hash == content.hash && reservation.is_confirmed_at(now))
            {
                confirmed.insert(reservation.provider);
            }
            let needed = target
                .min_replicas
                .saturating_sub(confirmed.len())
                .min(target.upper_bound().saturating_sub(known.len()))
                .min(
                    if content.repair_after.is_none_or(|retry_at| now >= retry_at)
                        && self
                            .repair_owner
                            .is_none_or(|owner| self.repair_policy.owns(content.hash, owner))
                    {
                        usize::MAX
                    } else {
                        0
                    },
                );

            let mut eligible = providers
                .iter()
                .filter(|&provider| {
                    provider.eligible_at(now)
                        && !known.contains(&provider.provider)
                        && !confirmed.contains(&provider.provider)
                })
                .cloned()
                .collect::<Vec<_>>();
            eligible.sort_by(|left, right| {
                right
                    .topology
                    .preference_score()
                    .cmp(&left.topology.preference_score())
                    .then(
                        right
                            .score
                            .cmp(&left.score)
                            .then(left.provider.as_bytes().cmp(right.provider.as_bytes())),
                    )
            });
            let mut selected_providers = HashSet::new();
            let requests = eligible
                .into_iter()
                .filter(|provider| selected_providers.insert(provider.provider))
                .take(needed)
                .map(|provider| ReplicationRequest {
                    hash: content.hash,
                    provider: provider.provider,
                })
                .collect();

            statuses.push(ReplicaStatus {
                hash: content.hash,
                known_replicas: known.len(),
                confirmed_replicas: confirmed.len(),
                requests,
            });
        }

        Ok(ReplicationPlan { target, statuses })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iroh::SecretKey;

    fn provider(seed: u8) -> PublicKey {
        SecretKey::from_bytes(&[seed; 32]).public()
    }

    fn hash(seed: u8) -> Hash {
        Hash::from_bytes([seed; 32])
    }

    fn state(seed: u8, score: u64) -> ProviderState {
        ProviderState {
            provider: provider(seed),
            lease_expires_at: 100,
            available: true,
            capacity_available: true,
            score,
            topology: ProviderTopology::default(),
        }
    }

    #[test]
    fn target_selection_prefers_specific_scope_then_priority() {
        let global = ReplicaTarget::new(ReplicaScope::global(), 1, Some(2), 100).expect("valid target");
        let community = ReplicaTarget::new(
            ReplicaScope {
                community: Some("lan-a".to_owned()),
                ..ReplicaScope::global()
            },
            2,
            Some(3),
            1,
        )
        .expect("valid target");
        let controller = ReplicationController::new(vec![global, community]);

        let scope = ReplicaScope {
            community: Some("lan-a".to_owned()),
            ..ReplicaScope::global()
        };
        assert_eq!(controller.target_for(&scope), controller.targets.get(1));
        assert_eq!(
            controller.target_for(&ReplicaScope::global()),
            controller.targets.first()
        );
    }

    #[test]
    fn rejects_inverted_replica_bounds() {
        let result = ReplicaTarget::new(ReplicaScope::global(), 3, Some(2), 0);
        assert!(result.is_err());
    }

    #[test]
    fn reconciliation_honors_confirmed_minimum_and_known_maximum() {
        let target = ReplicaTarget::new(ReplicaScope::global(), 3, Some(4), 0).expect("valid target");
        let controller = ReplicationController::new(vec![target]);
        let content = ReplicaContent::new(hash(1), vec![provider(1)], vec![]);
        let plan = controller
            .reconcile(
                &ReplicaScope::global(),
                10,
                &[content],
                &[state(2, 10), state(3, 20), state(4, 30)],
            )
            .expect("matching target");

        let status = plan.statuses.first().expect("reconciliation should produce a status");
        assert_eq!(status.known_replicas, 1);
        assert_eq!(status.confirmed_replicas, 0);
        assert_eq!(
            status
                .requests
                .iter()
                .map(|request| request.provider)
                .collect::<Vec<_>>(),
            vec![provider(4), provider(3), provider(2)]
        );
    }

    #[test]
    fn topology_hints_prefer_direct_and_local_providers() {
        let target = ReplicaTarget::new(ReplicaScope::global(), 2, None, 0).expect("valid target");
        let controller = ReplicationController::new(vec![target]);
        let content = ReplicaContent::new(hash(5), Vec::new(), Vec::new());
        let direct = ProviderState {
            topology: ProviderTopology {
                interface: Some("lan0".to_owned()),
                subnet: Some("lan-a".to_owned()),
                latency_ms: Some(10),
                path: ProviderPath::Direct,
            },
            ..state(1, 1)
        };
        let relay = ProviderState {
            topology: ProviderTopology {
                latency_ms: Some(1),
                path: ProviderPath::Relay,
                ..ProviderTopology::default()
            },
            ..state(2, 100)
        };
        let plan = controller
            .reconcile(&ReplicaScope::global(), 10, &[content], &[relay, direct])
            .expect("matching target");

        assert_eq!(
            plan.requests()
                .iter()
                .map(|request| request.provider)
                .collect::<Vec<_>>(),
            vec![provider(1), provider(2)]
        );
    }

    #[test]
    fn reconciliation_rejects_expired_unavailable_and_full_providers() {
        let target = ReplicaTarget::new(ReplicaScope::global(), 2, None, 0).expect("valid target");
        let controller = ReplicationController::new(vec![target]);
        let content = ReplicaContent::new(hash(2), vec![provider(1)], vec![provider(1)]);
        let mut expired = state(2, 20);
        expired.lease_expires_at = 10;
        let mut unavailable = state(3, 30);
        unavailable.available = false;
        let mut full = state(4, 40);
        full.capacity_available = false;

        let plan = controller
            .reconcile(&ReplicaScope::global(), 10, &[content], &[expired, unavailable, full])
            .expect("matching target");
        assert!(plan.requests().is_empty());
    }

    #[test]
    fn reconciliation_deduplicates_provider_observations() {
        let target = ReplicaTarget::new(ReplicaScope::global(), 2, None, 0).expect("valid target");
        let controller = ReplicationController::new(vec![target]);
        let content = ReplicaContent::new(hash(4), vec![], vec![]);
        let plan = controller
            .reconcile(
                &ReplicaScope::global(),
                10,
                &[content],
                &[state(2, 20), state(2, 10), state(3, 5)],
            )
            .expect("matching target");

        assert_eq!(plan.requests().len(), 2);
        assert_eq!(
            plan.requests().first().expect("first request should exist").provider,
            provider(2)
        );
        assert_eq!(
            plan.requests().get(1).expect("second request should exist").provider,
            provider(3)
        );
    }

    #[test]
    fn active_unconfirmed_reservation_occupies_capacity_but_not_minimum() {
        let target = ReplicaTarget::new(ReplicaScope::global(), 2, Some(2), 0).expect("valid target");
        let controller = ReplicationController::new(vec![target]);
        let content = ReplicaContent::new(hash(5), vec![], vec![]);
        let reservation =
            ProviderReservation::new(hash(5), provider(2), 5, 20, false, true).expect("valid reservation");
        let plan = controller
            .reconcile_with_reservations(
                &ReplicaScope::global(),
                10,
                &[content],
                &[state(2, 20), state(3, 10)],
                &[reservation],
            )
            .expect("matching target");

        let status = plan.statuses.first().expect("reconciliation should produce a status");
        assert_eq!(status.known_replicas, 1);
        assert_eq!(status.confirmed_replicas, 0);
        assert_eq!(
            status.requests,
            vec![ReplicationRequest {
                hash: hash(5),
                provider: provider(3),
            }]
        );
    }

    #[test]
    fn confirmed_reservation_satisfies_minimum_and_expired_reservations_do_not() {
        let target = ReplicaTarget::new(ReplicaScope::global(), 1, None, 0).expect("valid target");
        let controller = ReplicationController::new(vec![target]);
        let content = ReplicaContent::new(hash(6), vec![], vec![]);
        let confirmed = ProviderReservation::new(hash(6), provider(2), 5, 20, true, true).expect("valid reservation");
        let active = controller
            .reconcile_with_reservations(
                &ReplicaScope::global(),
                10,
                std::slice::from_ref(&content),
                &[],
                &[confirmed],
            )
            .expect("matching target");
        assert_eq!(
            active
                .statuses
                .first()
                .expect("active status should exist")
                .confirmed_replicas,
            1
        );
        assert!(active.requests().is_empty());

        let expired = ProviderReservation::new(hash(6), provider(2), 5, 10, true, true).expect("valid reservation");
        let after_expiry = controller
            .reconcile_with_reservations(&ReplicaScope::global(), 10, &[content], &[], &[expired])
            .expect("matching target");
        let status = after_expiry.statuses.first().expect("post-expiry status should exist");
        assert_eq!(status.known_replicas, 0);
        assert_eq!(status.confirmed_replicas, 0);
    }

    #[test]
    fn rejects_non_expiring_reservations() {
        assert!(ProviderReservation::new(hash(7), provider(1), 10, 10, false, true).is_err());
    }

    #[test]
    fn repair_jitter_is_deterministic_and_bounded() {
        let policy = RepairPolicy {
            jitter_window: 30,
            ownership_buckets: 4,
        };
        let first = policy.retry_at(hash(8), 100);
        assert_eq!(first, policy.retry_at(hash(8), 100));
        assert!((100..=130).contains(&first));
        let owner_specific = policy.retry_at_for(hash(8), provider(8), 100);
        assert!((100..=130).contains(&owner_specific));
        assert_eq!(
            RepairPolicy {
                jitter_window: 0,
                ownership_buckets: 1,
            }
            .retry_at(hash(8), 100),
            100
        );
    }

    #[test]
    fn ownership_sampling_is_stable() {
        let policy = RepairPolicy {
            jitter_window: 0,
            ownership_buckets: 7,
        };
        let owner = provider(9);
        assert_eq!(policy.owns(hash(9), owner), policy.owns(hash(9), owner));
        assert!((1..=32).any(|seed| policy.owns(hash(9), provider(seed))));
        let non_owner = (1..=32)
            .map(provider)
            .find(|candidate| !policy.owns(hash(9), *candidate))
            .expect("at least one provider should be outside the hash bucket");
        let target = ReplicaTarget::new(ReplicaScope::global(), 1, None, 0).expect("valid target");
        let controller = ReplicationController::new(vec![target]).with_repair_policy(policy, Some(non_owner));
        let deferred = controller
            .reconcile(
                &ReplicaScope::global(),
                10,
                &[ReplicaContent::new(hash(9), vec![], vec![])],
                &[state(2, 10)],
            )
            .expect("matching target");
        assert!(deferred.requests().is_empty());
        assert!(
            RepairPolicy {
                jitter_window: 0,
                ownership_buckets: 0,
            }
            .owns(hash(9), owner)
        );
    }

    #[test]
    fn stale_replicas_no_longer_block_repair() {
        let target = ReplicaTarget::new(ReplicaScope::global(), 2, Some(2), 0).expect("valid target");
        let controller = ReplicationController::new(vec![target]);
        let content = ReplicaContent::new(hash(10), vec![provider(1)], vec![provider(1)]);
        let mut stale = state(1, 100);
        stale.lease_expires_at = 10;
        let plan = controller
            .reconcile(&ReplicaScope::global(), 20, &[content], &[stale, state(2, 50)])
            .expect("matching target");

        let status = plan.statuses.first().expect("reconciliation should produce a status");
        assert_eq!(status.known_replicas, 0);
        assert_eq!(status.confirmed_replicas, 0);
        assert_eq!(
            plan.requests(),
            vec![ReplicationRequest {
                hash: hash(10),
                provider: provider(2),
            }]
        );
    }

    #[test]
    fn repair_after_and_owner_policy_defer_requests() {
        let target = ReplicaTarget::new(ReplicaScope::global(), 1, None, 0).expect("valid target");
        let policy = RepairPolicy {
            jitter_window: 0,
            ownership_buckets: 1,
        };
        let controller = ReplicationController::new(vec![target]).with_repair_policy(policy, Some(provider(1)));
        let content = ReplicaContent::new(hash(11), vec![], vec![]).with_repair_after(20);
        let providers = [state(2, 10)];

        let before_retry = controller
            .reconcile(&ReplicaScope::global(), 19, std::slice::from_ref(&content), &providers)
            .expect("matching target");
        assert!(before_retry.requests().is_empty());

        let after_retry = controller
            .reconcile(&ReplicaScope::global(), 20, &[content], &providers)
            .expect("matching target");
        assert_eq!(after_retry.requests().len(), 1);
    }

    #[test]
    fn rerunning_reconciliation_reflects_membership_changes() {
        let target = ReplicaTarget::new(ReplicaScope::global(), 1, None, 0).expect("valid target");
        let controller = ReplicationController::new(vec![target]);
        let content = ReplicaContent::new(hash(3), vec![], vec![]);
        let first = controller
            .reconcile(
                &ReplicaScope::global(),
                10,
                std::slice::from_ref(&content),
                &[state(2, 10)],
            )
            .expect("matching target");
        assert_eq!(first.requests().len(), 1);

        let second = controller
            .reconcile(&ReplicaScope::global(), 101, &[content], &[state(2, 10)])
            .expect("matching target");
        assert!(second.requests().is_empty());
    }
}
