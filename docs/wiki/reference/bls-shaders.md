# Retail BLS shader research

`scripts/bls_dump.py` extracts retail GXSH bundles for offline reference. It reports signatures and declared resource slots, writes one DXBC container per permutation, and disassembles SM5 through the MIT-licensed `dxbc` 0.1.0 crate. The engine runtime does **not** load these files.

## Usage

Build the standalone helper once (no system package installation; four pinned Cargo dependencies):

```sh
scripts/agent/agent-run bls-reader cargo build --locked \
  --manifest-path scripts/bls-disasm/Cargo.toml --target-dir target/bls-disasm
python3 scripts/bls_dump.py 2977134 -o data/bls-research/dumps/fel \
  --casc-local ../asset-resolver/target/release/casc-local
python3 scripts/bls_dump.py data/bls-research/2977223.bls \
  -o data/bls-research/dumps/procwaterabove
python3 scripts/test_bls_dump.py -v
```

FDIDs use `data/bls-research/<fdid>.bls`; absent files are extracted from **local CASC only**, using the explicitly supplied `--casc-local` binary or `CASC_LOCAL` environment variable. Launch extraction through `agent-run` under agent rules. The reader does not download assets or build tools automatically. `--cache` and `--disassembler` override the default paths. Tests require FDIDs 2977281, 2977223, 2977134, 2977131 and 2990816. Missing assets fail rather than skip.

Each output directory contains `manifest.json` and `permutation-NNNN.dxbc`, `.shex`, `.asm`. DXIL gets the container and manifest only. The manifest records original/program SHA-256, permutation offsets, opaque keys and wrapper bytes, DXBC chunk tags, input/output semantic names and masks, texture/sampler slots and constant-buffer vector counts. Slots are **declared bindings**, not proof that a runtime branch executes or that a particular image is bound. Assembly is the external output of the existing disassembler, not reconstructed HLSL. Decode warnings fail explicitly.

## Proven container layout

All integers below are little-endian u32. Offsets are file-relative unless stated. Verified on the three requested SM5 fixtures, the local liquid-name sweep and Fel DX60.

| Offset | Field | Evidence / meaning |
| --- | --- | --- |
| 0x00 | `HSXG` bytes | GXSH magic in little-endian storage |
| 0x04 | `0x1000e` | Accepted version; others fail |
| 0x08 | profile FourCC | `05XD` bytes mean DX50; `06XD` mean DX60 |
| 0x0c | permutation table offset | 40 in tested bundles |
| 0x10 | permutation count | Includes aliases, not just unique programs |
| 0x14 | compressed-block table offset | `40 + count*24 + 4` |
| 0x18 | block count | Number of independent zlib streams |
| 0x1c | compressed body offset | Block table offset + `(block_count+1)*4` |
| 0x20 | unknown flags/version word | 1 in tested DX50; 65537 in Fel DX60; semantics unknown |
| 0x24 | unknown word | 0 in tested bundles |

Each 24-byte permutation entry stores inflated-body offset, record length, and 16 opaque bytes. The bytes differ between programs, but neither a hash algorithm nor a permutation-key interpretation is proven. Preserve them; do not label them feature bits. The next u32 after the entries is total inflated-body size.

The block table contains `block_count+1` offsets relative to the compressed body. First is zero, last equals compressed-body length. Each interval is one complete zlib stream producing 16 KiB, except the final shorter block. Programs can cross block boundaries. The concatenated inflated body is indexed by permutation entries, **not** by scanning compressed bytes. Distinct records cover the body exactly; identical offset/length aliases are valid (ffxunderwaterlow contains two entries pointing at one 616-byte record).

Permutation records have an opaque wrapper around a conventional DXBC container. The tested SM5 liquid records start DXBC at byte 96. The tool finds DXBC within each bounded record, validates the container/chunk sizes and retains prefix/suffix bytes. It does not claim that the wrapper fields are understood. Extracted DXBC headers include their original checksum. ISGN/OSGN describe signatures; SHEX/SHDR contain SM4/5 tokens. RDEF is absent in the tested liquid SM5 programs. Slot reports decode `dcl_resource`, `dcl_sampler` and `dcl_constantbuffer` operands; constant-buffer sizes are float4 vector counts. Fel DX60 has SFI0, ISG1, OSG1, PSV0, HASH and DXIL chunks.

## Liquid findings (verified 2026-11-05)

Local CASC resolution-cache build key: `bc493ecb2898766c0f8947a99b4b3d65`. This identifies the source cache, not a numeric retail build. Names come from the existing local community listfile; no ocean/slime/shore-named BLS entry exists there. This does not prove that those effects do not exist under other shader names.

| Bundle / FDID | Permutations | Texture slot count range | t5 indices (zero-based) |
| --- | ---: | ---: | --- |
| procwaterabove / 2977223 | 54 | 6–12 | all |
| procfel / 2977215 | 12 | 6–11 | 2–5, 8–11 |
| procwaterbelow / 2977225 | 18 | 4–9 | all |
| waterfall / 2977284 | 2 | 0–7 | 0 |
| procmagma / 2977219 | 4 | 5–6 | none |
| fel / 2977134 | 4 | 3–5 | none |
| water / 2977281 | 6 | 2–5 | none |
| mediumwater / 2977186 | 6 | 2–5 | all |
| mediumwaterbelow / 2977187 | 6 | 2–5 | all |
| ffxunderwaterhigh / 2977130 | 2 | 4–5 | none |
| procwaterabovefog / 2977224 | 3 | 2–4 | none |
| magma / 2977160 | 4 | 2–3 | none |
| ffxwaterwindow / 2977132 | 3 | 2–3 | none |
| ffxunderwaterlow / 2977131 | 2 | 2 | none |
| waterripples / 2977283 | 1 | 1 | none |
| waterfogpoly / 2977282 | 1 | 0 | none |

Fel permutations 0/2 declare t0,t1,t16. Permutations 1/3 additionally declare t14,t15. This matches the independent decompilation finding. Water uses t1,t16 with optional t14,t15 and t13; it does not bind t5.

Procfel permutation 2 gives a concrete shore candidate:

```
uv6 = (TEXCOORD5.xy / TEXCOORD5.w) * cb1[14].xy + cb1[14].zw
D = saturate((sample(t6,s6,uv6).x - TEXCOORD0.z) * cb1[12].y)
shore = sample(t5,s5,float2(TEXCOORD1.z,1-D))
color = shore.rgb + (base_color-shore.rgb)*(D*shore.a)
```

It also propagates shore.a through the subsequent cb5 weighted accumulation into output alpha. Final t16 screen-volume compositing changes RGB only: `accumulated.rgb*(1-S.a)+S.rgb`. Opaque shore texture alpha therefore still permits the depth-difference transition. The CPU-bound identity of t6, cb1[12].y and the rule selecting procfel are **not** established by this shader alone. Constant indices address float4 vectors, not byte offsets.

## Limits

- No original resource, variable or constant-buffer member names; semantic names such as TEXCOORD are retained.
- No runtime constant values, texture identities, sampler state, blend/depth/raster state or CPU permutation-selection policy.
- DXIL containers/signatures are listed, but no DXIL assembly or slot reflection is implemented. Unknown slots are `null`, not an empty list.
- Only version 0x1000e with the verified table layout is accepted. Unsupported indexed declarations, OSG5 signatures, corrupt compression and invalid extents fail explicitly. This is a research reader, not a universal BLS specification.
- No system packages installed. PyPI queries found no `dxbc`, `pydxbc` or `dxbc-disassembler`; no vkd3d-compiler/dxc/fxc binary was present. crates.io supplied an existing disassembler, so no speculative SM5 decoder was written.

## Sources

- [`bls_dump.py`](../../../scripts/bls_dump.py), [`bls-disasm`](../../../scripts/bls-disasm/Cargo.toml), [`fixture tests`](../../../scripts/test_bls_dump.py) — executable parser and behavioral evidence.
- [dxbc 0.1.0](https://crates.io/crates/dxbc/0.1.0) — MIT package metadata; SM5 decoding/formatting. Cargo.lock pins exact dependencies/checksums.
- Local `data/community-listfile.csv`, extracted FDIDs and generated manifests — shader names, byte layouts and resource declarations.

## See Also

- [CASC extraction](../../casc-extraction.md) — persistent local asset extraction.
- [Liquid rendering contract](../../specs/liquid-rendering.md) — runtime scope remains separate from offline research.
