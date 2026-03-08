# Login UI Porting Brief

Status as of 2026-03-08. Resume from here if context is lost.

## What's Done

### Editbox nine-slice textures
- `set_editbox_backdrop()` sets `frame.nine_slice` with `Common-Input-Border.blp` (128×32)
- `edge_size: 12.0`, `text_insets: [12.0, 5.0, 0.0, 5.0]`
- Committed as `21046d7f5`

### WoW-identical anchoring
- Editboxes: 320×42, chained via TOP→BOTTOM with 30px gaps
- Labels: children of editboxes, BOTTOM→TOP with y_offset=23.0
- ServerInput: CENTER on root, y_offset=80.0
- UsernameInput: TOP→ServerInput BOTTOM, y_offset=-30.0
- PasswordInput: TOP→UsernameInput BOTTOM, y_offset=-30.0
- LoginButton: TOP→PasswordInput BOTTOM, y_offset=-50.0
- SaveCheckbox: TOP→LoginButton BOTTOM, y_offset=-10.0
- CreateAccountButton: BOTTOM→root BOTTOM, y_offset=54.0 (centered)
- MenuButton: BOTTOM→CreateAccountButton TOP, y_offset=10.0 (centered)
- ExitButton: BOTTOMRIGHT→root BOTTOMRIGHT, offset (-10, 16)
- `set_anchor()` helper for arbitrary anchor points
- Committed as `992fd17d1`

### Text rendering fixes
- `text_transform()` uses `text_insets` from EditBoxData instead of hardcoded 4.0px
- `text_anchor()` sets `Anchor::CENTER_LEFT`/`CENTER`/`CENTER_RIGHT` based on JustifyH
- Committed as `992fd17d1`

### Screen size sync
- `sync_screen_size` system reads actual window dimensions into registry
- `login_sync_root_size` keeps root frame sized to window
- Fixes bottom elements being off-screen when window != 1920×1080
- Committed as `992fd17d1`

## Remaining Issues

### Nine-slice border thickness
- Combined texture is 128×32, individual parts are 64×16 each
- edge_size=12 gives 12px rendered border — may be correct but looks thin
- The texture content layout within 128×32 needs visual inspection
- **TODO**: Render the BLP to PNG and inspect, or try edge_size=16

### Label positioning
- Labels sit in the 30px gap between editboxes — correct per WoW layout
- But visually they appear below their editbox rather than clearly above
- Could increase gap or adjust y_offset for clarity

## Key Files

- `src/login_screen.rs` — all login UI construction and interaction
- `src/ui/render_text.rs` — text positioning (text_transform, text_anchor, text_insets)
- `src/ui/render_nine_slice.rs` — nine-slice sprite rendering
- `src/ui/render.rs` — sync_button_nine_slices, is_renderable, UiQuad rendering
- `src/ui/layout.rs` — resolve_anchors, resolve_frame_layout
- `src/ui/plugin.rs` — sync_screen_size system
- `src/ui/frame.rs` — Frame, NineSlice, Backdrop, WidgetData structs

## Y-offset Sign Convention

Our layout resolver: `target_y = ay - y_offset`
- **Positive y_offset** → moves UP (smaller screen Y)
- **Negative y_offset** → moves DOWN (larger screen Y)
- WoW XML y values map as: our_y_offset = wow_xml_y (same sign for up=positive)
- Exception: `set_layout_anchor` negates y: `y_offset: -y` (converts screen-down to WoW-up)

## WoW Reference (AccountLogin.xml)

```
AccountEditBox:   CENTER on parent, y=50 (50px above center)
PasswordEditBox:  TOP→AccountEditBox BOTTOM, y=-30
LoginButton:      TOP→PasswordEditBox BOTTOM, y=-50
CreateAccount:    BOTTOM→ExitButton TOP, y=10
Menu:             BOTTOM→CreateAccount TOP, y=10
Labels:           BOTTOM→editbox TOP, y=-23 (64px tall in WoW, 20px in ours)
EditBox size:     320×42
TextInsets:       left=12 right=5 bottom=5
```

Source: `/home/osso/Projects/wow/wow-ui-sim/Interface/BlizzardUI/Blizzard_GlueXML/Mainline/AccountLogin.xml`
