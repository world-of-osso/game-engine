# Local Bevy transform patch

`bevy_transform/` is the unmodified crates.io `bevy_transform` 0.19.0 source at import, retaining its licenses. `Cargo.toml` patches that exact package locally and includes it as a workspace member so its real contention/transform tests use the root build profile and target cache. Other Bevy crates remain registry dependencies.

Patch: the transform worker uses ordinary blocking mutex acquisition instead of a contended `try_lock` spin loop. In the current full-client CPU sample, transform worker polling accounted for 19.46% of sampled CPU; 90.05% of that function's samples fell on the lock-retry branch. Evidence: `data/diagnostics/cpu-goal-resumed/current-baseline/transform-hot-instructions-correct-symbol.log`.

Worker count, queue/batch ownership, hierarchy traversal, and completion ordering are unchanged. No production sleeps, frame caps, or worker limits are added. A Linux regression holds the receiver mutex for 300 ms, checks that the real propagation worker consumes at most two thread CPU ticks, and confirms queued child transforms complete after release. The original spin loop consumed 29 ticks. Native CPU comparison remains required; this contention regression is not full-client performance proof.

Retire this override when an upstream Bevy release removes the measured lock spinning and passes the contention, transform-lifecycle, and native CPU checks for this workload.
