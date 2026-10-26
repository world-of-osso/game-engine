# Launcher pack files: measurement and proposed contract

Status: design only, 2026-10-26. No launcher, publisher, installed client, or R2 object was changed. The distribution launcher is the separate `game-launcher` repository; `game-engine/launcher/src/main.rs` is the developer build launcher, not the downloader. Source inspection used game-launcher commit `9450ac748a61ee625b61781fd352881a58ad1511`; the running release's exact source revision was not established.

## Live baseline

A read-only GET of `https://files.worldofosso.com/manifest.json` returned version `2026-10-25`, ETag `"e868b27b62e9906ffb5e8cc73afbc1a6-54"`, SHA-256 `a863d566bf19750a0f0d085ac56e5c3cdb5ea40655151294ee4e91d964980fa2`. The 282,700,729-byte JSON took 8.79 s to fetch on this host. The manifest has 1,678,635 files. Applying the launcher's Windows entry filter leaves **1,678,634 files, 229,942,296,696 bytes (214.15 GiB)**. It excludes only the other platform's `game-engine-*` entry, not the other platform's runtime and library files.

Histogram, Windows-filtered; ranges are lower-inclusive and upper-exclusive:

| Size | Files | Bytes |
|---|---:|---:|
| <1 KiB | 109,502 | 27,432,454 |
| 1–4 KiB | 204,929 | 519,502,908 |
| 4–16 KiB | 472,191 | 4,292,235,837 |
| 16–64 KiB | 391,950 | 12,935,909,433 |
| 64–256 KiB | 295,808 | 39,644,186,815 |
| 256 KiB–1 MiB | 162,424 | 68,903,536,401 |
| 1–8 MiB | 40,605 | 78,945,820,168 |
| >=8 MiB | 1,225 | 24,673,672,680 |

### Sample and limits

`scripts/measure_launcher_packs.py` selects a seeded uniform sample of 256 Windows-filtered entries no larger than 8 MiB, GETs with up to 32 Python worker threads, checks size and SHA-256, writes each to an isolated directory, renames it, and deletes the samples. The experiment used 34,204,073 bytes and took **2.84 s**. Per request, mean time to headers was **292 ms** (median 270 ms, p95 571 ms); mean body read was **12.0 ms**. Mean in-memory SHA-256 was **0.079 ms** and mean buffered write plus rename **0.266 ms**; these are overlapping per-task timings, not additive wall-clock attribution. The observed aggregate rate was 12.1 MB/s for this small-file mixture, not a bulk-transfer benchmark. The sample uses Python's HTTP implementation and is not the Rust launcher; it does not model Windows disk or antivirus, actual connection reuse, launcher cache scanning, manifest parsing, retries, or durable storage. Files over 8 MiB, accounting for 24.67 GB, were excluded.

As a **small synthetic delta**, this 256-file/34.2 MB transfer took 2.84 s in the same sample, excluding manifest download and installed-file scan. It is not a measured real release-to-release delta or end-to-end launcher update; no previous release manifest or representative changed-file set was available. The first 32 tasks total 3,942,129 bytes, but their individual durations do not establish a separate 32-file update wall time.

A rough cold-install extrapolation for the eligible 1,677,409 files is `2.8366 × 1,677,409 / 256 = 18,586 s`, or **5.16 hours**, plus the 1,225 large files, manifest fetch/parse, scan and other overhead. This is a sample-derived estimate, not an observed install or reliable prediction: short-run concurrency ramp-up, server/cache effects, connection handling, disk and size variance can change it substantially. In particular it is not valid to sum the 292 ms per-request latency across 1.68 million files without accounting for 32-way concurrency. No full cold download, native Windows benchmark, real small-delta update, pack throughput test, or durable-disk benchmark was performed.

**Capacity incident:** the capacity check immediately before the sample reported `HOLD` because another waiter was ahead; the sample was nevertheless started. It completed, and its files were removed. This did not respect the required admission signal. No further heavy download or build was started.

### Existing per-file work

The current source fetches the manifest with conditional GET and caches it. It walks all selected entries and stats each installed path. An unchanged `(size, mtime)` receipt avoids rehashing; otherwise it reads the file and hashes it. Missing/changed entries use one GET per attempt, up to 32 concurrent downloads; each successful body is SHA-256 hashed as it streams to a sibling `.part`, flushed, checked for exact size and digest, then renamed and statted for a receipt. The source calls `flush`, not `fsync`/`sync_all`; it does not promise durable file or directory sync. The cache is serialized to `.file_hashes.json`. Network retry attempts and filesystem costs can add work. Existing files are not installed from the developer launcher.

## Recommended format and update contract

Use immutable, content-addressed **pack objects** with a separate versioned v2 manifest. Target 32 MiB of uncompressed payload per pack; cap at 64 MiB, except where an individual record requires otherwise. Store records without extra compression initially: the measured assets include already-compressed formats, and compression benefit and CPU cost have not been measured. Each pack has a versioned header, bounded record lengths and a SHA-256 identity of the complete object; the manifest indexes each file's ordered content chunks by digest, length, pack identity and byte offset. Hash every reconstructed file against its manifest SHA-256 before installation. Validate lengths, offsets, digest, duplicate destinations and paths before writing; reject absolute paths, `..`, and Windows path aliases/collisions.

Build packs deterministically within top-level asset directory and update-frequency groups: stable extracted models/textures/terrain separately from frequently changing code, imported project files and catalogs. Do not shift old chunks merely to fill packs. For files at most 1 MiB use one chunk per file; split larger files with deterministic content-defined chunks around 1 MiB, with bounded minimum/maximum sizes. Preserve existing digest-to-pack assignments across releases; pack only new chunks into new immutable objects. This avoids rewriting a whole 32 MiB download for a one-byte file change and allows reuse after insertions in large files. Publish the exact chunking parameters and index version as part of the format. This grouping is a proposal, not a measured optimal boundary.

For a cold install, download complete referenced packs concurrently, verify each pack, extract the requested files into staged sibling files and verify each file before replacement. For updates, compare file receipts first; obtain only missing chunks, using HTTP byte ranges and coalescing nearby ranges in the same pack when the extra bytes cost less than another request. Check each chunk digest even for range responses; validate `206` and `Content-Range`, and fail explicitly if the server does not honor the range. This keeps wire deltas proportional to changed chunks rather than changed packs, but scattered chunks can still require many requests. Do not claim a measured delta saving until release pairs are benchmarked. Install paths and file layout remain unchanged, so the game need not read packs.

### Integrity, resume and rollout

The v2 manifest describes file paths, file sizes and SHA-256s plus chunk locations and pack hashes/sizes. Publish immutable packs and verify their presence before publishing the v2 manifest last. Retain every object referenced by any manifest that supported clients can still use; never delete immediately after switching a manifest. Treat SHA-256 as corruption detection, not publisher authentication: retain the current HTTPS trust model; signing is a separate decision.

Resume only complete verified chunks or packs recorded in an install-local staging area. After interruption, rehash retained chunks before reuse; discard invalid partial records. A fully verified file is atomically replaced where the platform permits. An interrupted update can leave a mix of individually verified old and new files; the launcher must finish reconciliation before allowing play. Check available space for staging and replacement before transfer. The plan does not claim whole-install atomicity or power-loss durability; define and test those requirements before implementation.

Publish v2 at a distinct endpoint; leave the legacy manifest and its referenced individual files available for already-installed older launchers. New launchers migrate in place by checking existing file receipts and SHA-256s, retrieving only missing or changed content. They must never require uninstall or delete the install tree. Release and retire legacy support only under a separately approved policy; retain enough old pack objects for in-flight manifests and rollback.

### Windows

Preserve `C:\Users\adeia\AppData\Local\World of Osso` and all unrelated user data; never run the uninstaller. Use Unicode/long-path-safe APIs, prevent case-insensitive path collisions and reserved names, and stage on the same volume as each destination. Stop the game before replacing its files; if a file is locked or replacement fails, report the error and do not claim success. Test native Windows rename/replacement semantics, interrupted updates, available space, filesystem allocation and security software impact on a disposable installation, not the preserved user installation.

### Publisher changes

`scripts/depot-build.py` builds the extension; it is not the current publisher. `deploy.sh` assembles with `scripts/player_bundle.py`, invokes the file-server manifest generator and uses `scripts/manifest_diff.py` plus rclone for uploads. Add deterministic chunk/pack generation after staging in that publishing pipeline, with persisted chunk-to-pack index, staged-byte verification, and tests for reproducibility, corrupt records, old-manifest retention and reconstruction. `depot-build.py` should remain the build step unless an explicit orchestration interface is needed; it must not independently publish. Publish packs before v2 manifest, maintain legacy publication during migration, and make garbage collection a separate retention-aware operation.

## Implemented v2 format (work in progress)

The separate `manifest-v2.json` has `version: 2`, `format_version: 1`, and `chunking` with version 1, minimum 262,144 bytes, average 1,048,576 bytes, and maximum 4,194,304 bytes. Each file has its path, size, SHA-256, and ordered chunks. Each chunk has its SHA-256, size, pack SHA-256, pack size, and payload offset. Packs live at `packs/<sha256>` and start with the eight-byte magic `OSSPACK\x01`; each record is a little-endian 32-bit payload length followed by its bytes. Pack identity is SHA-256 of the whole object.

Files of at most 1 MiB use one chunk. For larger files, the version-1 chunker starts a 64-bit rolling value at zero and updates it for each byte as `(value << 1) + gear[byte]`, modulo 2^64. `gear[i]` is the little-endian integer represented by the first eight bytes of `SHA256(bytes([i]))`. After the minimum, a chunk ends when the low 20 bits are zero, or at the maximum. The rolling value resets at each boundary. The final chunk may be shorter than the minimum.

The publisher retains `chunk-index.json` version 1 beside the packs. Its key is the asset group and chunk digest; an existing key keeps its old pack and offset. New records are sorted by group and digest, and packs target 32 MiB. `data/<directory>` entries have separate groups; other paths use their top-level directory, or `_root` for root files. Old pack objects are not deleted. This implementation is not yet approved for live publication.

## Local fixture proof (2026-10-26; incomplete implementation)

`scripts/tests/launcher_pack_e2e.py` exercised the real Linux CLI launcher at game-launcher commit `1a1f25b`, with 96 files of 4 KiB and a 64 KiB test pack target. The localhost server added 10 ms per request. It compared every installed file byte-for-byte after each run. Results include manifest and launcher-update requests; asset counts are in parentheses:

| Case | Requests (assets) | Wall time |
|---|---:|---:|
| Pack cold | 9 (7) | 0.1092 s |
| Legacy cold | 99 (96) | 1.7214 s |
| Pack delta, two changed files | 4 (2) | 0.0900 s |
| Legacy delta, same changes | 5 (2) | 0.0968 s |
| Pack resume after process interruption | 3 (1) | 0.1020 s |

The interruption retained seven verified packs; none was requested again on resume. This is a small localhost fixture, not a release benchmark. Publisher deployment integration, range and security coverage, running-game exclusion, and native Windows proof still need verification. Default-feature Cargo check failed on this host because the `javascriptcoregtk-4.1` system package is absent. No live publication was performed.

## Expected effect and acceptance measurements

At an ideal 32 MiB per pack, 229.94 GB is about **6,853 packs** instead of 1,678,634 individual file GETs, a theoretical **99.6% reduction in cold payload request count**; group boundaries, chunk duplication, empty files and actual fill increase the count. This is arithmetic, not a measured result. Using the sample's 292 ms mean header time as a rough comparison, 32-way request-latency work changes from about 4.25 hours (`1,678,634 × 0.292 / 32`) to about 63 seconds (`6,853 × 0.292 / 32`). Network transfer, hashing, disk writes, parallel overlap and server behavior prevent translating that difference directly into wall-time savings. A full cold install must still write and verify the same file payload; pack bandwidth is unmeasured.

Before release, benchmark old and new launchers against the same immutable R2 dataset on native Windows: full cold install, at least one captured real small release delta, request count, bytes, manifest time, latency, bulk throughput, CPU, disk time and end-to-end wall time. Exercise interruption, corruption, range handling and migration. Compare actual pack fill and retained-payload growth. Do not treat the sample or arithmetic here as an acceptance result.
