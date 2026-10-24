# Benilla / game-engine BLP Fields

BLP1/BLP2 serialized inputs, transient decoder representations and texture-output boundaries. E's CPU mip-0 API is distinct from its compressed DXT1/3/5 full-chain uploader and alpha-BC1 RGBA-chain path. B's selected parser is BLP2-only; its Vanilla-client comments do not establish cross-version or runtime compatibility.

Snapshot: **2026-10-24**. Reference pins inherited from [[benilla-wmo-root-fields]]: E native baseline `354bdb7dbf177047ed46d63168acec531f5c4426`; B `8a6c74036cfe3d00275fd75ad1aeae1f4ea7d0df`. These identify the comparison baseline, not a newly verified checkout hash. Static local text inspection only: no source execution, imports, builds, tests, network, downloads, extraction or nested agents. No runtime/visual compatibility claim.

**Roots:** E=`/syncthing/Sync/Projects/world-of-osso/game-engine`; B=`/home/osso/Repos/benilla`; D=`/home/osso/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/image-blp-1.2.0/src`. **Anchors:** H=`D/parser/header.rs`; T=`D/types/header.rs`; P=`D/parser/direct/mod.rs`; P1=`D/parser/direct/blp1.rs`; P2=`D/parser/direct/blp2.rs`; J=`D/parser/jpeg.rs`; C=`E/godot/core/src/blp.rs`; Q=`E/godot/core/src/asset/blp_format.rs`; N=`E/godot/rust/src/assets/material.rs`; B0=`B/crates/benilla-blp/src/lib.rs`; BF=`B/crates/benilla-formats/src/lib.rs`. Offsets below are absolute file byte offsets, all multibyte scalar inputs little-endian. Named symbols supplement line ranges.

## Individual header inputs and version gates

H:14–101 produces `BlpHeader { version, content, flags, width, height, mipmap_locator }` (T:47–54). This object survives within dependency decoding, not as E's public `RgbaImage`/`GpuImage` metadata. B0:163–207 builds a private borrowed `Header`; its fields similarly do not survive as serialized metadata in public outputs.

| Serialized input | E dependency retention / actual interpretation | B retention / bounded difference |
|---|---|---|
| Magic `u8[4] @0` | H:14–15,105–117 validates magic and retains normalized version. BLP1 takes `Old`, BLP2 takes `Blp2` flags; raw magic lost. | B0:177–180 requires BLP2 and at least148 bytes. BLP1/JPEG support is not established by this parser. |
| Content `u32 @4` (both) | H:16–23; T:25–34:0→JPEG,1→Direct; unknown warns and selects JPEG. `parser/mod.rs::parse_content` selects payload parser from this tag. | B0:180–181 deliberately does not read it; comment assumes Direct for shipped1.12 art. No content value retained, no JPEG dispatch. |
| BLP1 alpha depth `u32 @8` | H:44–61 normalizes invalid values to0: JPEG accepts0/8, Direct0/1/4/8. Retained in `Old.alpha_bits`; original invalid value lost. | Version rejected before this layout is parsed. Do not treat four BLP2 bytes as this word. |
| BLP2 compression `u8 @8` | H:24–32; T `Compression::try_from`:0 JPEG,1 Raw1,2 DXTc,3 Raw3; unknown fails despite log wording mentioning a default. P:23–145 dispatches Direct variants; Direct+compression0 explicitly errors. | B0:182,266–289,307–333:1 Raw1,2 DXT,3 Raw3; other values error. Header stores raw byte transiently. |
| BLP2 alpha depth `u8 @9` | H:33–43 stores raw byte. T `BlpFlags::alpha_bits` returns it except Raw3 returns4 independently of this byte. Raw1 converter accepts0/1/4/8; C:157–159 tests depth>0 for BC1 alpha-chain conversion. | B0:183 stores as `u32`; Raw1 `alpha_at` consumes it. DXT codec selection does not use this field in the inspected implementation. |
| BLP2 alpha type `u8 @10` | H:34–43 retains; P:70–145:DXTc0→DXT1/BC1,1→DXT3/BC2,7→DXT5/BC3; other values error in dependency. C:45–92 intercepts11 as BC5 before dependency parsing. | B0:184,239–248:1→BC2,7→BC3,all others→BC1. This permissive legacy selection is not E behavior, nor BC5 support. |
| BLP2 mip flag `u8 @11` | Retained byte; any nonzero true (T `has_mipmaps`). Q:3–9 clears only private-copy flag for CPU decoding; native DXT parses original flag. | B0:185 derives bool and then `chain`; raw byte lost. |
| Width `u32 @12` (both) | H:72 retained; drives derived level widths and decoded allocation. C output retains width, subject to DXT truncated-level adjustment below. | B0:186 retained; >8192 rejected; zero passes header check. Drives per-level sizes. |
| Height `u32 @16` (both) | H:73 retained independently; drives derived level heights and allocation. JPEG result dimensions come from decoded JPEG, not an E header copy. | B0:187 retained; same cap/check distinction as width. |
| BLP1 extra `u32 @20` | H:74–83 retains `Old.extra`; not an encoding selector in inspected Direct/JPEG decoding. No independent E output/consumer established. Do not invent flags from this word. | BLP1 rejected. |
| BLP1 mip flag `u32 @24` | H:74–83 retains; any nonzero true. Q:10–12 writes zero to private copy; C CPU conversion remains mip0. | BLP1 rejected. |

BLP2 fixed header ends at148; BLP1 ends at156. BLP0 external mip locators are outside this page's scope; dependency recognition of BLP0 does not establish an E buffer-only external-file workflow (D/parser/mod.rs:30–66,69–90).

## Mip directories: each indexed input, not sixteen new field names

For every `i=0..15`, these are separate serialized words. Formula notation specifies each exact offset/type, including unused entries. H:120–128 reads **all** words into `MipmapLocator::Internal { offsets:[u32;16], sizes:[u32;16] }`; B0:188–192 reads all into private vectors. Neither E output nor B public texture output retains file positions.

| Individual directory input | BLP1 offset | BLP2 offset | Interpretation / retention boundary |
|---|---|---|---|
| `offsets[i]`, `u32` | `28+4*i` (28…88) | `20+4*i` (20…80) | Absolute address into original bytes, not relative to palette/JPEG header. Dependency checks offset and offset+size against file length for selected levels (P1:15–35; P2:20–40,87–109; J `read_image`). B0 `level_data` addresses the same absolute location. |
| `sizes[i]`, `u32` | `92+4*i` (92…152) | `84+4*i` (84…144) | Declared level span. Dependency parses needed pixels/alpha/whole blocks within it, not arbitrary opaque tail retention. B DXT may read beyond a short declared span, described below. |

Derived dimensions are `(max(width>>i,1), max(height>>i,1))` for deeper levels; level0 uses original dimensions (T:73–87; B0 `level_size`). Dependency `mipmaps_count` is `max(floor(log2(width)),floor(log2(height)))` when flag true; parsers read level0 plus levels1 through that count, capped at16 total (T:56–66; P1:48–55; P2:58–73,130–137; J internal branch). These counts are helpers, not serialized fields.

B0:151–158 derives the same logarithmic `chain` but `decode`/`decode_native` iterate `0..chain.clamp(1,16)`, **not count+1** (B0:266–305,307–347). The accessor count is therefore not proof that every mathematically possible level through1×1 was read. Zero offset or size at a deeper B level ends the chain; mip0 is still attempted. Dependency Raw3 stops at a deeper zero size; Raw1/DXT/JPEG loops do not share that explicit stop rule. These are literal loop boundaries, not a runtime correctness verdict.

## Palette, JPEG and raw pixel inputs

| Serialized input / gate | E transient representation, loss and consumer | B handling |
|---|---|---|
| Direct palette entry `u32[j]`, `j=0..255` | BLP1 `156+4*j`; BLP2 `148+4*j`; BGRA bytes. P:23 reads256 words before Direct compression dispatch, including unused DXT/Raw3 palette. `BlpRaw1.cmap` feeds conversion; `BlpDxtn.cmap`/Raw3 palette survive dependency parsing but are not E upload metadata. | B0:193–195 requires1024 bytes at148 for all accepted compression values, even DXT. Borrowed palette only Raw1 consumes. |
| Palette B byte `+0`, G `+1`, R `+2`, A `+3` per entry | C:22–30 swaps B/R in Raw1 words before dependency conversion, whose low byte is treated as red. Separate alpha plane determines output alpha; palette A is not copied to output. Retire normalization when dependency handles BGRA. | B0:371–387 copies R/G/B from byte2/1/0 and ignores palette A. No E-style preliminary swap needed. |
| Raw1 index `u8[k]` | Level start+k, `k=0..w*h-1`; P1:37–45 retains `Raw1Image.indexed_rgb`; conversion indexes palette; index representation lost in RGBA. | B0:371–387 reads same index stream, errors if shorter than pixel count; outputs only RGBA. |
| Raw1 separate packed alpha | Starts after `w*h` indices; length `ceil(w*h*alpha_bits/8)` (P1:40–45), retained `indexed_alpha`. Depth0 yields RGB→opaque RGBA;1 reads low-bit-first values0/1;8 copies bytes. Dependency4-bit branch assigns the **whole byte** for even pixels and byte>>4 for odd pixels, without nibble replication (D/convert/raw1.rs:28–112). C's generic alpha repair is not a four-bit normalization. | B0:389–412:1-bit low-first→0/255;4-bit low/high nibble→nibble×17;8-bit direct;other depths opaque. Missing1/4-bit alpha bytes act as0, missing8-bit as255. No blanket all-zero→opaque pass in these routines. |
| Raw3 pixel `u32[k]` (BLP2 compression3) | Level start+4*k; P2:42–53 retains `Raw3Image.pixels`; D/convert/raw3.rs:25–34 maps low BGRA bytes→RGBA. Palette unused; header alpha-depth accessor's4 does not mean four-bit stored pixels. | B0:414–425 maps each four-byte BGRA pixel to RGBA, keeps pixel A; errors for short pixel span. |
| JPEG common-header size `u32` | At156 for BLP1,148 for BLP2 **when content selects JPEG**. J:21–29 warns if >624, does not cap; reads `size+2` bytes following word. Raw size scalar not retained separately. | No JPEG path in B0 accepted compression dispatch; no BLP1 layout. |
| JPEG common-header bytes `u8[size+2]` | BLP1 starts160; BLP2 starts152. J retains `BlpJpeg.header`. `D/types/jpeg.rs::full_jpeg` excludes the final2 bytes and concatenates common header with selected body. Those2 bytes are implementation-treated padding, not an invented serialized field. | No corresponding representation in inspected B0. |
| JPEG per-level body bytes | Directory-selected offset/size; J internal branch copies into `BlpJpeg.images[i]`. D/convert/jpeg.rs:11–18 decodes reconstructed JPEG, converts to RGBA and swaps R/B. Header alpha-depth is not passed as a separate alpha plane to this converter. JPEG header/body bytes are lost in E image output. | Not supported by this selected BLP2 Direct parser. No claim about every B utility or original-client version. |

`D/types/image.rs` groups `BlpImage { header, content }` with content variants JPEG, Raw1, Raw3, DXT1/3/5. `Raw1Image { indexed_rgb,indexed_alpha }`, Raw3 words, DXT block vectors and JPEG header/body vectors are **decoded/helper representations of the inputs above**, not extra on-disk headers. E discards them after constructing pixels or upload data.

## DXT blocks, BC5 exception and E output boundaries

Dependency P2:87–127 takes selected directory span, computes `ceil(w*h/16)*block_bytes`, and if short reads only complete blocks available inside that span. This pixel-area formula is not identical to independently rounding both block-grid axes for every non-square sub4 level. E uses its own `ceil(w/4)*ceil(h/4)` block requirement for upload; do not infer universal malformed/non-square-file acceptance.

| Encoding input | Actual selected interpretation / retained result |
|---|---|
| DXT1 / BC1 payload (BLP2 compression2,type0) |8 bytes per4×4 block: little-endian RGB565 endpoints at+0/+2 and packed2-bit color selectors at+4. Dependency keeps bytes in `DxtnImage.content`; E `BlockFormat::Dxt1` retains compressed bytes unless alpha depth>0 selects per-level RGBA decompression. No separate endpoint/selector output fields. |
| DXT3 / BC2 payload (compression2,type1) |16-byte block: first8 bytes explicit4-bit texel alpha, next8 BC color data. E retains block bytes and complete authored chain when available; zero-alpha policy below can rewrite first8 bytes. |
| DXT5 / BC3 payload (compression2,type7) |16-byte block: alpha endpoints bytes0/1, packed3-bit selectors bytes2…7, color block bytes8…15. C `bc3_alpha_is_zero` reads endpoints/selectors to detect all-zero decoded alpha; interpolation is a helper calculation, not another serialized field. |
| BC5 special selector | C:45–92 checks magic BLP2,compression byte@8=2,type byte@10=11 before dependency call. Requires header≥148,nonzero width@12/height@16; checks RGBA and block-size overflow. Content@4,alpha depth@9,mip flag@11 and palette are **not** dispatch/validation inputs of this special path. |
| BC5 directory/payload | Only offset0 `u32@20` and size0 `u32@84` are read; offset≥148,checked end within file,and span≥`ceil(w/4)*ceil(h/4)*16` required. C calls `texpresso::Format::Bc5.decompress`; two8-byte channel blocks decode to RGBA `(R,G,0,255)` per documented contract. No normal-Z reconstruction,compressed BC5 upload or authored deeper-level retention in this path. B0 has no BC5 variant;type11 takes its BC1 default, not equivalent decoding. |

The DXT block interpretation above describes codec inputs passed to the decoder, not a claim that E explicitly parses each color field itself. D/convert/dxtn.rs:6–27 passes `DxtnImage.content` and derived dimensions to the selected texpresso decoder; endpoint/color-selector internals are not independently audited here. C:173–232 and its block/alpha helpers expose these retention boundaries:

| API / helper output | Fields, consumers and losses |
|---|---|
| `decode_rgba` → `RgbaImage` | C:9–43 returns `pixels:Vec<u8>,width:u32,height:u32`. BC5 takes special mip0 path first. Other inputs are copied; Q clears BLP2 byte11 or BLP1 word24; dependency parses the modified copy; conversion explicitly selects level0. Original source bytes are not modified. Palette/JPEG/Raw3 and CPU DXT converge here; serialized directory,flags,palette and compressed representation are lost. |
| `fix_1bit_alpha` | Q:15–29:if maximum output alpha is0,sets every alpha255; if maximum is1,expands positive alpha to255. This global heuristic can alter authored all-zero alpha; it is not faithful retention of every zero/one alpha value. BC5 returns before this pass. |
| `decode_gpu` → `GpuImage` | C:125–170 retains `format:GpuFormat,width,height,mipmaps:bool,data:Vec<u8>`. Original-byte DXT1/3/5 parsing supports full authored mip chains. Non-DXT uses CPU RGBA output and `mipmaps=false`; BC5 does likewise directly. `GpuFormat` has Dxt1,Dxt3,Dxt5,Rgba8,not BC5. |
| `compressed_image` | C:173–204 requires level0; `level_zero_size` halves both dimensions while data cannot fit and both sides>4. Derives expected sizes down to1×1; if all levels/bytes present,concatenates exact required spans with `mipmaps=true`. Otherwise emits level0 only,false. Extra level bytes are not preserved. This is conditional completeness, not a claim that every input has a complete chain. |
| `decompress_bc1` | C:207–232:alpha-depth>0 DXT1 is expanded **for every retained level** via BC1 decoding to RGBA8,preserving dimensions/mipmap flag. Full alpha-BC1 RGBA chains are supported; this is not CPU `decode_rgba`'s mip0-only path. Source comments explain Godot's BC1 RGB upload distinction; no new driver/runtime validation here. |
| `make_zero_alpha_opaque` | C helper scans all retained DXT3/5 blocks; if all decoded alpha is zero,rewrites alpha blocks to opaque. DXT3 gets8×0xff;DXT5 endpoints255/255 and zero selectors. DXT1 untouched. Hence “compressed as authored” has this explicit byte-retention exception. |

## Located consumers and B representations

N:542–550 `load_texture` returns CPU `(pixels,width,height)` for overlay composition; `base_texture` selects shared texture when no overlays,otherwise composes pixels and uploads RGBA (N:553–580). `shared_texture` N:585–605 calls `decode_gpu`,caches by texture key; `insert_shared_texture` accepts previously decoded `GpuImage`. N:630–648 maps Dxt1/3/5/Rgba8 to Godot DXT1/3/5/RGBA8 and passes dimensions,**mipmap flag and concatenated data** to `Image::create_from_data`,then `ImageTexture`. This is a concrete native full-chain consumer,not merely a retained parser vector.

CPU consumers also include `E/godot/rust/src/terrain/textures.rs:78–100::load_image`,which caches `Arc<RgbaImage>` by FDID,and `E/godot/rust/src/ui/assets.rs::decode_blp` (call at176). `assets/creature.rs:338` and `spell_assets.rs:218` locate other `decode_gpu` calls. This is a bounded consumer inventory,not proof that these are the only texture paths.

B0:45–158 defines `DecodedBlp { width,height,mips:Vec<MipLevel>,mip_chain_count }`,each `MipLevel { width,height,rgba }`; `NativeBlp { width,height,texels,mips:Vec<NativeMip>,mip_chain_count }`,each `NativeMip { width,height,bytes }`. `BlpTexels` is Rgba8Unorm/Bc1/Bc2/Bc3. File offsets,palette and alpha control bytes are not public fields. `decode_native` keeps DXT block form;Raw1/Raw3 produce RGBA. `decode_level` can expand one block level to RGBA (B0:307–360).

B0:210–237 `dxt_level_span` first bounds-checks declared span; if shorter than required,it can consume following file bytes from that level offset. `pad_to` (B0:362–369) then zero-pads/truncates to exact required size; this is not strict byte-for-byte retention of a short level. Reference-address comments describe B's intended1.12 model and explicitly distinguish padding from reference stale-buffer reads; not independent authentication here. B's Raw1 alpha and DXT selection differences must stay version/implementation-bounded.

BF:498–544 `BlpMipChain { width,height,texels,mips }` derives per-level sizes; `into_rgba8` calls `decode_level`. BF:563–615 adapters call `decode`/`decode_native`,project each level to its pixel/block buffer and take at most `mip_chain_count().max(1)` levels. These concrete adapters consume B outputs; their names/comments saying “every authored level” do not override the loop/count boundary documented above. Renderer-wide original-client parity is unmeasured here.

## Scope and remaining gaps

Individual header fields,each indexed directory word,palette channel/index/alpha inputs,JPEG header/body split and DXT/BC5 selection/output boundaries are covered by static source evidence. No extra reserved-header member or opaque unknown-byte preservation is established. Unknown BLP2 compression/type behavior is explicitly implementation-specific above.

No executed fixtures,real-file acceptance census,visual proof,driver-format verification,original-client authentication or universal cross-version compatibility is supplied. Exact BC5 channel decompression internals reside in texpresso,not image-blp; this pass establishes E's call/input contract,not a separate codec audit. B renderer-wide upload/alpha behavior beyond the located format adapters remains outside this bounded slice. Environment fields are excluded.

## Sources

- E [core decoder](../../../godot/core/src/blp.rs),[private-copy/alpha helpers](../../../godot/core/src/asset/blp_format.rs),[native material binding](../../../godot/rust/src/assets/material.rs),[terrain CPU consumer](../../../godot/rust/src/terrain/textures.rs) — actual parsing adaptations,retention and upload boundary.
- D `parser/{header,direct/mod,direct/blp1,direct/blp2,jpeg}.rs`,`types/{header,image,jpeg}.rs`,`convert/{raw1,raw3,jpeg,dxtn}.rs` under the absolute dependency root above — actual image-blp1.2.0 parser/converter inputs,not generic format documentation.
- B `crates/benilla-blp/src/lib.rs`,`crates/benilla-formats/src/lib.rs` — accepted version,private header,decoded/native/helper representations and adapter consumers.
- Local leads `/tmp/blp-env-final.md`,`/tmp/blp-e-field-map.md`,`/tmp/blp-environment-integration.md`,`/tmp/blp-environment-residual.md` — BLP portions only; primary source above supersedes broad/stale claims. Residual note supplies no additional BLP field evidence.

## See Also

- [[benilla-asset-field-inventory]] — parent comparison and evidence vocabulary.
- [[benilla-wmo-root-fields]] — shared baseline pins; separate WMO inventory.
- [[blp-format]] — texture pipeline and Raw1 BGRA normalization context.
- [[godot-dxt1-punch-through]] — alpha-BC1 RGBA-chain motivation.
- [[open-source-wow-clients]] — reference-client versions and comparison limits.
