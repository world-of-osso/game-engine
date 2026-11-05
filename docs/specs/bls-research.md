# Offline BLS research

## Required behavior

- Read a local retail BLS bundle or resolve an FDID through an explicitly supplied local CASC extractor; never use the Blizzard CDN.
- Parse the verified GXSH 0x1000e layout, preserve unproven fields, inflate independent 16 KiB zlib blocks and extract a DXBC file for each permutation, including aliases.
- Report input/output signatures and declared SM5 texture, sampler and constant-buffer slots. Produce readable SM5 assembly with an existing disassembler. List DXIL without claiming disassembly or slot reflection.
- Reject corrupt/truncated data and unsupported layouts explicitly. Missing fixtures must fail tests, not skip.
- Verify water (2977281), procwaterabove (2977223) and Fel (2977134), including Fel's exact t0/t1/t16 and optional t14/t15 declarations. Rank the local liquid-name pixel DX50 bundles by texture-slot count and identify t5 candidates.

## Exclusions

The engine runtime must not load BLS. Resource/member names, runtime constants, binding identities, render state and CPU permutation-selection rules cannot be inferred from these containers alone.

See [format, usage and limits](../wiki/reference/bls-shaders.md).
