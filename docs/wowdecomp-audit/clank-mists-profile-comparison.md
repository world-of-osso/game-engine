# Clank Together target vs. local Mists profile

Verified: 2026-10-24. Browser page snapshot: `2026-10-24T06:08:44Z`; API snapshot: `2026-10-24T06:09:33Z`. Source/repository inspection only; no client or executable was run.

## Target identity

The Clank Together `wowdecomp` project reported target ID `15890-x64`, label `World of Warcraft 5.0.3 (build 15890, x64)`, SHA-256 `819519af4eb6c46f72e4e9c151c78b6b33cb4a232602e5fab61d29143e4c9cc0`, and toolchain `msvc2010-x64`. The public [`/api/info`](https://saty.dev/api/info) reported repository head `9a47a653926c6c9b21a65655f9d10387a74b0af5` at the API snapshot time.

The linked repository's initial-import commit identifies it as a standalone 5.0.3/build 15890 port copied from the `wowdecomp15595` working tree at `92c679d2` (`w15595`), including uncommitted changes with x64 addresses resolved. That is source provenance, not evidence that the target executable is build 15595. The target metadata is 15890. The configured target hash was not independently compared with an independently sourced original client binary.

Evidence: [project initial import](https://saty.dev/git/clank/wowdecomp/commit/19d92d8e9fd4b82f65f532a5d50b9ef079f67fd2); [project](https://saty.dev/git/clank/wowdecomp); [live site](https://saty.dev/#/).

## Progress metric observed

The live page displayed **48.53% of functions** (20,251 / 41,730) and **41.02% of matched code bytes** (4,140 / 10,092 KB). The matched-code figure includes peer-confirmed code; “about 40%” is not function-count completion. Both percentages describe Clank's own 15890-x64 target.

At the API snapshot time, public [`/api/progress`](https://saty.dev/api/progress) returned 41,730 functions and 10,334,086 bytes, with `exact` 11,561 / 1,166,911 bytes, `match_unresolved` 174 / 129,788 bytes, and `confirmed` 8,516 / 2,942,521 bytes. Summing these status buckets gives 20,251 functions and 4,239,220 bytes, matching the page's rounded headline percentages. The lower task-state view separately showed 20,392 `done`; that is not the headline matched-function numerator. No cause for the difference between these live views was established, so preserve the snapshot boundary and do not substitute task-state totals for the displayed progress metric.

The site's own description says submissions are compared byte-for-byte and its function map is in address order. The percentages are project-specific matched-code progress, not a general “Mists profile completeness” score.

## Comparison boundary

The local simulator feature is named `client-mists` (`/syncthing/Sync/Projects/wow/wow-ui-sim/Cargo.toml:158–159`). Its vendored Mists UI-source snapshot reports `5.5.4.68042` (`vendor/wow-ui-source-mists/version.txt:1`; snapshot commit `0fa050554e3e3c8ad559ff258516d9c86b70815`, recorded in `cf2-client-api-audit-20261023.md`). That is UI-source provenance, not a verified native executable identity. At that inspection, the audit recorded no local native Mists implementation or executable pin. A subsequent source refresh cloned Clank's 15890 tree; see [the pinned loader follow-up](mists-sharedxml-loader-15890.md). This does not change the local simulator UI-source identity.

Thus Clank's **5.0.3/build 15890 x64 executable target is not the same version as the local 5.5.4.68042 Mists UI-source snapshot**. There is no evidence mapping the local simulator profile to that target binary. Do not report Clank's 41.02% as coverage of our local Mists profile. A direct comparison needs a verified native executable/profile ID and matching target hash for the local profile.

## Evidence class and limits

- **Repository-backed:** Clank's import commit labels the port 5.0.3/build 15890 and describes its source origin.
- **Point-in-time public observation:** `/api/info`, `/api/progress`, and the live page supplied the target metadata and progress values above; live feeds have no absolute timestamp in the displayed page.
- **Not established:** independent authenticity of the target executable hash; any native executable corresponding to local `client-mists`; runtime equivalence between either project and an official client.
