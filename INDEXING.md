## Opt-In Indexing Service (`syncweb indexing`)

To ensure the core `iroh-blobs` and `iroh-docs` sync engine remains lean and strictly focused on peer-to-peer file synchronization, advanced discovery and resilience features are implemented as an opt-in layer called the **Indexing Service**. 

This service runs independently (often in the same binary, but asynchronously) and subscribes to events from the core engine. It manages its own SQLite database for full-text search (FTS) and metadata tracking, ensuring that core data synchronization is never blocked by complex querying or network health monitoring.

An indexer may be:

  - local and private;
  - hosted by a community;
  - federated with other trusted indexers; or
  - populated from public gossip/DHT announcements.

### 1. Discovery and Catalogs
Instead of forcing the core to understand federated catalogs, users can explicitly publish folder metadata to a catalog namespace.
*   **Action:** When a user opts a folder into public discovery, the indexing service reads the files and publishes `CatalogRecords` (title, tags, hashes) to a dedicated public `iroh-docs` catalog namespace.
*   **Search:** The service maintains a local SQLite FTS5 index of any catalogs the user subscribes to, entirely outside the core file-sync path.
*   **Overlap Note:** Core `syncweb find` performs local regex/glob searches on the filesystem or synced namespaces. `syncweb indexing search` performs global queries across published catalogs even for files you have not downloaded.

### 2. Resilience and Availability
Instead of the core managing complex replication leases, the indexing service acts as an automated fleet manager.
*   **Action:** Users configure a folder with a replication budget (e.g., "ensure 3 providers"). The indexing service monitors the network for signed `ProviderLeases`.
*   **Execution:** If availability drops below the threshold, the indexing service commands the core engine to fetch and pin the blob. The core engine simply sees a standard "download and pin" request.
*   **Thundering Herd Mitigation:** To prevent all peers from fetching simultaneously when availability drops, the system uses **consistent hashing** (only peers mathematically closest to the blob's hash are responsible), **randomized jitter** (staggered fetch delays), and **gossip short-circuiting** (if a peer gossips a new `ProviderLease` during the delay, others cancel their fetch).
*   **Overlap Note:** Core `syncweb health` shows basic local observations of peers. `syncweb indexing health` shows cryptographically verified leases and historical uptime.

### 3. Web of Trust (WoT) Metadata
Instead of formal, heavy compute pipelines (like OCR and PDF extractors) running automatically on all clients, metadata extraction is crowdsourced to trusted entities.
*   **Action:** Trusted authors in a Web of Trust (WoT)—whether humans or automated bots—can manually append metadata, tags, or derivatives to a file's record.
*   **Execution:** These metadata entries are synced via `iroh-docs` and indexed by the local indexing service. You only index metadata written by authors you trust.
*   **Overlap Note:** Core `syncweb stat` shows raw file sizes and hashes. `syncweb indexing meta` surfaces community-curated metadata (like transcriptions or content tags).

### CLI Subcommands (`syncweb indexing`)
*   `syncweb indexing enable <folder>` - Opt a folder into the indexing service.
*   `syncweb indexing publish <folder> --catalog <name>` - Publish to a catalog.
*   `syncweb indexing search "query"` - Search across known catalogs (FTS).
*   `syncweb indexing health <hash>` - Check verified leases and availability.
*   `syncweb indexing meta add <hash> <key> <value>` - Append WoT metadata to an entry.
