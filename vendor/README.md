# Temporary Bevy task-submission experiment

`bevy_ecs` and `bevy_tasks` are unmodified 0.19.0 registry sources at initial import, retaining upstream licenses and normalized manifests. Local patches test bulk task submission without serializing systems, removing work, changing pool sizes, or changing dependency versions/features.

Experiment contract: [task submission batching](../docs/specs/task-submission-batching.md). Remove these local patches and vendor directories if the experiment fails behavior or performance acceptance.
