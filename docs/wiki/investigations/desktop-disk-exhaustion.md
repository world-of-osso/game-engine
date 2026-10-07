# Desktop disk exhaustion

Observed OssoBuild recovery on Windows local date 2026-10-06 separates BuildKit cache reclamation, WSL guest free space and Windows host capacity. Recovery completed; persistent GC deployment and long-term stability remain unverified.

## Observed exhaustion

Windows C: had 66 MB free; `ext4.vhdx` occupied 1,012,488,208,384 bytes (approximately 1 TB). WSL reported 911 GiB used and 46 GiB free; BuildKit reported 482.7 GB reclaimable. Multiple Python, tmux and docker-init SIGBUS-named crash dumps appeared together. Disk exhaustion is observed; the full causal kernel chain linking it to those crashes was not independently proven.

## Completed recovery

Temporary swaps were cleared after shutdown. BuildKit prune with `--max-used-space 100GB` removed 426.1 GB; the subsequent cache measurement was 56.56 GB. Guest `fstrim` reported 470.1 GiB. Offline DiskPart compaction completed, with its process absent at the completion observation:

- VHD: 607,486,214,144 bytes.
- Windows C: free: 405,526,130,688 bytes (approximately 377.7 GiB).
- Remounted Linux: 513 GiB used, 444 GiB free.

Guest deletion/trim and offline VHD compaction were distinct recovery stages; guest free space alone did not establish reclaimed Windows capacity. VM memory remained 20 GB. Laptop cache (9.93 GB) and free space (210 GiB) were untouched.

## Prevention boundary

[[build-hosts#Builder GC policy]] owns the user-approved aggregate budget replacing warm-every-slot retention. Source configuration is updated, but persistent deployment and sustained behavior are not proven by this recovery.

## Sources

- Windows `Get-PSDrive C`, VHD file size and `%LOCALAPPDATA%\Temp\wsl-crashes` inventory; Linux `df -hT /`; `docker buildx du --builder game-engine` — live capacity and crash evidence on Windows local date 2026-10-06.
- `docker buildx prune --builder game-engine --all --force --max-used-space 100GB`, `/usr/sbin/fstrim -v /`, then `wsl.exe --shutdown` and DiskPart `compact vdisk` — recovery commands; post-compaction file size and free space were measured after DiskPart exited.
- [BuildKit configuration](../../../scripts/depot/buildkitd.toml) — persistent aggregate GC policy.

## See Also

- [[build-hosts]] — GC ownership and historical eviction diagnosis.
- [Build guide](../../remote-builds.md#warm-slot-rule) — fixed-slot cache identities and ordinary-agent restrictions.
