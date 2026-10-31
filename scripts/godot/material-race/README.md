# Material queue race stress

Run with a Godot 4.7.2 editor built with `dev_build=yes` and the additional
SelfList linkage assertions from the Stonevault diagnostic's
`dev-build/instrument-self-list.py`. This is a standalone project; it does not
load the game client or select a UI skin. It requires a real RenderingDevice,
not `--headless`. On the agent host, for example:

```
scripts/agent/agent-run material_race /home/osso/.worktrees/client-slot.sh \
  timeout 240 env WLR_BACKENDS=headless WLR_LIBINPUT_NO_DEVICES=1 WLR_RENDERER=vulkan \
  cage -- "$GODOT_DEV_BIN" --path scripts/godot/material-race \
  --display-driver wayland --rendering-method forward_plus --audio-driver Dummy \
  --script res://stress.gd
```

The project selects the separate rendering thread. Four workers create and
release ShaderMaterials in batches while frames advance. A pass prints the
material and frame counts and exits zero; assertions, other errors, crashes,
and timeouts are failures. This is a timing-sensitive stress test, not a proof
that every unpatched run must fail.
