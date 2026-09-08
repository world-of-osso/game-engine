# Local Bevy transform patch

`bevy_transform/` is the unmodified crates.io `bevy_transform` 0.19.0 source at import, retaining its licenses. `Cargo.toml` patches that exact package locally and includes it as a workspace member so its real contention/transform tests use the root build profile and target cache. Other Bevy crates remain registry dependencies.

Purpose: test replacing the transform worker's contended `try_lock` spin loop with ordinary blocking mutex acquisition. In the current full-client CPU sample, transform worker polling accounted for 19.46% of sampled CPU; 90.05% of that function's samples fell on the lock-retry branch. Evidence: `data/diagnostics/cpu-goal-resumed/current-baseline/transform-hot-instructions-correct-symbol.log`.

The patch must preserve worker count, queue/batch ownership, hierarchy traversal, and completion ordering. It must not impose frame caps, sleeps, or worker limits. Native CPU comparison is required; importing the dependency is not performance proof.

Retire this override when an upstream Bevy release removes the measured lock spinning and passes the contention, transform-lifecycle, and native CPU checks for this workload.
