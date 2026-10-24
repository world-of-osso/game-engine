# M2 Field Inventory: Scope and Static Evidence

Acceptance ledger for the source-grounded M2/SKIN comparison, not runtime compatibility certification. The inventory covers the selected reader/model/adapter source scope in [[benilla-m2-field-census]], expands implicit header/track inputs in the handling worksheets and separates raw/cooked/default/helper representations. WMO, terrain, texture and environment inventories remain independent unfinished work.

Verified: **2026-10-24**. E native baseline `354bdb7dbf177047ed46d63168acec531f5c4426`; B `8a6c74036cfe3d00275fd75ad1aeae1f4ea7d0df`. Native source was unchanged through the accepted documentation revisions below. Benilla's1.12.1/build5875 layouts are not modern E layouts. No builds, tests, parser/client execution/imports, downloads, extraction, asset-corpus validation or visual comparison were performed.

## Coverage and disposition

| Surface | Exact inventory scope | Proof / residual boundary |
|---|---|---|
| Named source members | **90 file-qualified representations /630 individually named members**: E40/362, B50/268. Census names/types/source lines checked against tracked source; all90 mapped to handling worksheets. | Static census/role/route proof at `41147b6bc`, wording delta `17e94300e`. Counts are declarations, **not630 unique disk fields**. Other source files/types outside the declared census scope are not silently included. |
| Header/container | Individually listed E/B count/offset words, bounding fields, recognized chunk framing and file-ID/sidecar inputs; source-bounded unread intervals explicit. | Corrected header audit `49e92f9f9`; later model-bit consumer anchors accepted at `41147b6bc`. Unknown modern bytes are not assigned Vanilla semantics. |
| Geometry/material | Vertex attributes; external SKIN versus embedded M2View headers; raw submesh/batch fields; material/texture/UV lookup inputs and downstream B consumers. | Baseline plus geometry deltas through `3aec3673e`. Coordinate lookup reaches E sky UV handling; ordinary native shader selection follows its own inputs. Not a demonstrated rendering defect or parity claim. |
| Animation/scene | Bone/sequence fields and opaque spans, ordinary/material track policies, external payload selection; attachments/events/lights/cameras/collision; B lookup/raw-sequence appendix. | Family correction audits `27b092c16`, `3f0745102`; appendix/citation/flag deltas `d91fd210a`, `41147b6bc`; all accepted. Camera curves and selected payload consumers remain specific documented limits, not missing inventory rows. |
| Particles/ribbons and projections | Generator/lifetime/multitexture/EXP2/ribbon inputs and descriptors, B504-byte particle accounting/tested flag predicates; eight B bounds members and twelve marker/bone-channel projections. | Particle/ribbon deltas `7147031ab`, `87e058339`, `5b3520885`;20 projection derivations plus consumers accepted through `d39e8c649`. E shorter particle branch remains a conditional/partial decoder boundary, not certified layout support. |

## Independent static proof ledger

Each original audit covered offsets/types, actual reads, retention/loss/derivation, claimed production consumers and source anchors. Corrections were committed before bounded independent delta verification. Passing unchanged scope was preserved rather than repeatedly re-audited. No command in this ledger ran project source or tests.

| Gate | Accepted documentation revision / scope | Result |
|---|---|---|
| Ribbon | `87e058339`, corrected citations and B non-global sequence qualifier; unchanged original rows preserved | PASS |
| Particle | `7147031ab`, normalized timestamp/formula and citation corrections | PASS |
| B particle envelope/flags | Envelope extension `8d0848917`; predicate/consumer/zSource corrections `5b3520885` | PASS |
| Header | `49e92f9f9`, global-duration retention/cursor/sidecar wording; unchanged header map preserved | PASS |
| Animation baseline | `27b092c16`, opaque bytes explicitly unread, not retained; corrected anchors | PASS |
| Scene | `3f0745102`, camera116-byte accounting, B lossy event-bone narrowing, production lighting/collision evidence and descriptors | PASS |
| Geometry and added B consumers | Baseline/corrections through `57a043a73`; final coordinate-combo attribution `3aec3673e` | PASS |
| B lookup/raw-sequence appendix | Eight rows and60 named+8 opaque=68-byte accounting; extended retention/rebasing anchors `d91fd210a` | PASS |
| E flag additions | Six bone-selector rows and header global bits0x8/0x8000/0x20000 at `41147b6bc` | PASS |
| Declaration reconciliation | Tracked census/source compared independently,90 records/630 members, no missing/extra/duplicate/count/type/line mismatches;133 added links at `41147b6bc` | PASS |
| Representation exceptions | Helper provenance, roles/routes and mixed inputs at `41147b6bc`; packed-color versus f32-scale wording `17e94300e` | PASS |
| Individual projection members | All20 derivations/actual consumers; clock-wrap and ordinary-clip body/invocation citation delta `d39e8c649` | PASS |
| Documentation wiring | Map/census/parent/index/log links and scope vocabulary at `41147b6bc`; broader-format goal retained | PASS; stale parent status corrected in ledger integration |

Final delta receipts: coordinate `agent_476`, appendix/projection/flags `agent_481/488/499`, census/map `agent_487/500`, documentation wiring `agent_489`. Initial reports with findings do not independently count as PASS: the specific corrective deltas above close those findings. These are static review receipts, not execution logs.

## Limits that remain visible

- Native source contains animated point lights, full global-clock paths and modern texture/UV structures; do not describe entire systems as absent from narrower parser snapshots or bounded searches.
- E camera output is a snapshot, not retained/evaluated spline animation. B's curved camera tracks are a version-qualified semantic lead, not transferable offsets or proof E assets should use Vanilla FoV.
- E particle keyframed-track retention can discard all-single-key sequence constants; enabled0 can become a default. EXP2 can clear prior zSource tracks before chunk validation. These are source counterexamples, **not executed fixtures**.
- Unknown/unbound consumers remain explicitly scoped: some event payloads/raw batch metadata, particle dependency filenames/follow parameters/alpha cutoff and ribbon color index. A bounded unknown is not proof of global non-use.
- Unread/opaque bytes and unsupported/version-specific interpretations stay unassigned. Census reconciliation does not certify all possible file versions, flags, assets, malformed inputs, shaders or visual/timing behavior. It authorizes no engine implementation.

## Sources

- [[benilla-m2-field-census]] — literal member scope and source locations.
- [[benilla-m2-representation-map]] — all90 records, roles and source-input exceptions.
- [[benilla-m2-representation-fields]] — individual bounds and marker/animation projections.
- [[benilla-m2-header-fields]], [[benilla-m2-geometry-fields]], [[benilla-m2-animation-fields]], [[benilla-m2-scene-fields]], [[benilla-m2-particle-fields]], [[benilla-m2-ribbon-fields]] — version-qualified field handling and source anchors.
- Independent static audits/deltas identified by revision, scope and receipt above; comparisons used tracked source/Markdown and read-only Git metadata.

## See Also

- [[benilla-asset-field-inventory]] — parent comparison and remaining WMO/terrain/texture/environment work.
- [[open-source-wow-clients]] — reference-version and evidence policy.
