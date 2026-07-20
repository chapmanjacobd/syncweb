## Opt-In Indexing Service (`syncweb indexing`)

To ensure the core `iroh-blobs` and `iroh-docs` sync engine remains lean and strictly focused on peer-to-peer file synchronization, advanced discovery and resilience features are implemented as an opt-in layer called the Indexing Service. 

This service runs independently (often in the same binary, but asynchronously) and subscribes to events from the core engine. It manages its own SQLite database for full-text search (FTS) and metadata tracking, ensuring that core data synchronization is never blocked by complex querying or network health monitoring.

An indexer may be:

  - local and private;
  - hosted by a community;
  - federated with other trusted indexers; or
  - populated from public gossip/DHT announcements.

### 1. Discovery and Catalogs
Instead of forcing the core to understand federated catalogs, users can explicitly publish folder metadata to a catalog namespace.
*   Action: When a user opts a folder into public discovery, the indexing service reads the files and publishes `CatalogRecords` (title, tags, hashes) to a dedicated public `iroh-docs` catalog namespace.
*   Search: The service maintains a local SQLite FTS5 index of any catalogs the user subscribes to, entirely outside the core file-sync path.
*   Overlap Note: Core `syncweb find` performs local regex/glob searches on the filesystem or synced namespaces. `syncweb indexing search` performs global queries across published catalogs even for files you have not downloaded.

### 2. Resilience and Availability
Instead of the core managing complex replication leases, the indexing service acts as an automated fleet manager.
*   Action: Users configure a folder with a replication budget (e.g., "ensure 3 providers"). The indexing service monitors the network for signed `ProviderLeases`.
*   Execution: If availability drops below the threshold, the indexing service commands the core engine to fetch and pin the blob. The core engine simply sees a standard "download and pin" request.
*   Thundering Herd Mitigation: To prevent all peers from fetching simultaneously when availability drops, the system uses consistent hashing (only peers mathematically closest to the blob's hash are responsible), randomized jitter (staggered fetch delays), and gossip short-circuiting (if a peer gossips a new `ProviderLease` during the delay, others cancel their fetch).
*   Overlap Note: Core `syncweb health` shows basic local observations of peers. `syncweb indexing health` shows cryptographically verified leases and historical uptime.

### 3. Web of Trust (WoT) Metadata
Instead of formal, heavy compute pipelines (like OCR and PDF extractors) running automatically on all clients, metadata extraction is crowdsourced to trusted entities.
*   Action: Trusted authors in a Web of Trust (WoT)--whether humans or automated bots—can manually append metadata, tags, or derivatives to a file's record.
*   Execution: These metadata entries are synced via `iroh-docs` and indexed by the local indexing service. You only index metadata written by authors you trust.
*   Overlap Note: Core `syncweb stat` shows raw file sizes and hashes. `syncweb indexing meta` surfaces community-curated metadata (like transcriptions or content tags).
    • Signed Mutable Pointers & Manifests (Mechanical): These answer the question, "Who said what, and when?" They provide cryptographic proof that a specific identity published a pointer to a specific hash, and that they did so with a valid monotonic sequence number (preventing rollbacks). They are entirely unopinionated about whether the content is good.
    • syncweb trust (Contextual): This answers the question, "How do we feel about what they said?" It is the policy layer that evaluates whether a publisher is trusted by you or your community, whether the content has been flagged by a moderator, or if valid license attestations exist.
    • Self-Revocation (Mechanical): If a publisher wants to take down their own content, they publish a new Signed Mutable Pointer with an incremented sequence number pointing to a tombstone or empty manifest. The core resolver handles this automatically.
    • Takedowns/Filtering (Contextual): If a publisher distributes malware and refuses to take it down, the community moderator publishes a signed ModerationRecord against the publisher's identity or content hash. The syncweb trust layer intercepts the discovery and hides the content, even though the publisher's pointer remains mathematically valid.

### 4. Stable Links, Resolvers, and Mirrors
A direct blob ticket is useful for immediate transfer, but it is not a durable public reference. It lacks a stable name and provides no standard way to resolve a newer version or alternate mirror. The indexing service manages stable references, resolution, and mirrors.
*   Action: Users can create immutable links (`syncweb://content/<content-id>` or `syncweb://collection/<collection-id>@<version>`) and signed mutable links (`syncweb://name/<publisher>/<alias>`).
*   Execution: The resolver translates these references to a signed manifest and available providers. Mutable aliases contain a signed pointer to an immutable manifest and never rewrite the content addressed by an old link. Version pinning is always available. Public collections can advertise multiple providers and mirrors.
*   Security & Revocation: Signed name records use monotonic sequence numbers to prevent rollbacks. Private links remain capability-based, carrying read capabilities and expiration. Revoking a private link prevents new authorized fetches.
*   Overlap Note: Core provides direct single-peer blob tickets. The indexing layer provides stable names, verifiable resolution across multiple providers, and mirror fallback.

### 5. Denylists and Filtering
To keep the core engine lightweight (relying only on basic `PeerStats` and `FolderStats`), advanced filtering is handled by the indexing service via hooks. Other applications can build GUIs/TUIs on top of these hooks (similar to libtorrent-rasterbar), so no complex transfers UI is needed natively.
*   Action: Users can configure Device-Level, File-Level, and Hash-Level local denylists to block specific content or peers.
*   Execution: The indexing service hooks into the sync engine's discovery and fetch pipeline. When an intent to fetch is created, it is validated against the denylists.
*   Federated Filter Lists: Users can subscribe to federated, community-maintained filter lists (similar to uBlock Origin filter lists or PeerBlock). These are distributed as standard `iroh-docs` namespaces and automatically update the local indexing service's blocklist.
*   Overlap Note: The core engine simply respects the filtering decisions provided by the indexing hooks, remaining ignorant of complex rule evaluation or federated list syncing.

### 6. Trust, Governance, and Moderation
Content hashes prove integrity, but they do not prove accuracy, authorship, legality, or quality. Public and community catalogs require mechanisms for spam, abuse, takedown, and conflicting claims without relying on a single global authority.
*   Action: Users can attach signed attestations for publisher metadata, licenses, and provenance. Moderators can issue signed `ModerationRecord`s to hide, warn, quarantine, or restore content within a specific scope. Users can also optionally establish a web of trust by delegating trust to other publishers.
*   Execution: Trust policies are evaluated locally (scoped by network, folder, or file). Moderation hides or de-prioritizes records in an index; it does not rewrite or delete immutable content on other nodes. The indexing service evaluates these records to return a discovery decision (Show, Warn, Hide, Quarantine).
*   Overlap Note: Core `syncweb` ensures cryptographic integrity of content hashes. The indexing service evaluates contextual trust, licenses, provenance, and community moderation policies.

### CLI Subcommands (`syncweb indexing`, `syncweb link`, `syncweb mirror`, `syncweb trust`, `syncweb moderation`, `syncweb attest`, `syncweb report`)
*   `syncweb indexing enable <folder>` - Opt a folder into the indexing service.
*   `syncweb indexing publish <folder> --catalog <name>` - Publish to a catalog.
*   `syncweb indexing search "query"` - Search across known catalogs (FTS).
*   `syncweb indexing health <hash>` - Check verified leases and availability.
*   `syncweb indexing meta add <hash> <key> <value>` - Append WoT metadata to an entry.
*   `syncweb indexing filter add <type> <value>` - Add a hash, device, or file to the local denylist.
*   `syncweb indexing filter subscribe <url>` - Subscribe to a federated filter list.
*   `syncweb link create <file-or-collection>` - Create a stable pinned or mutable link.
*   `syncweb link resolve <url>` - Resolve a link to its manifest, sequence, and providers.
*   `syncweb link revoke <link>` - Revoke a private link.
*   `syncweb mirror add <collection> <provider>` - Register an alternate mirror provider.
*   `syncweb trust show <content-or-publisher>` - Show trust, license, provenance, and moderation state.
*   `syncweb trust delegate <publisher>` - Cryptographically delegate trust to another publisher (Web of Trust).
*   `syncweb attest <content> --license <license>` - Sign an attestation for content.
*   `syncweb report <record> --reason <reason>` - Submit a moderation report.
*   `syncweb moderation ls` - List moderation records and decisions.
*   `syncweb moderation hide <record>` - Hide a record based on local or community policy.
