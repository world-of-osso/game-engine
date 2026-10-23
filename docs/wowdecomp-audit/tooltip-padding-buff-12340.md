# Tooltip padding and aura UI files — build 12340

Source inspection: 2026-10-23. Reference W: `wowdecomp` revision `0e4e9fe0a4a9fa08027c5eed4b2c5e873739cfd3`, reconstructed WoW 3.3.5a/build 12340. No client or simulator was run. Results below separate reconstructed native-code behavior from the locally cached UI source, which is labeled Wrath 3.4.3. No matching shipped 12340 Lua/XML files were found in the inspected local source cache, so exact target-build UI-template/loading claims remain unverified.

## 1. Tooltip width arithmetic

**Source-backed:** `SetMinimumWidth(width [, force])` takes the width first and optional force flag second. `SetPadding(padding)` takes a single scalar; the binding converts it to tooltip layout units and stores `m_padding`. `ClearLines()` calls `ClearTooltip()`. `SetOwner()` also calls `ClearTooltip()`. `ClearTooltip()` resets a non-forced minimum width, but it does not reset `m_padding`; the tooltip constructor initializes `m_padding` to zero. The close-to-source setter for `m_padding` is `SetPadding`.

For the specified call order, `ClearLines()` happens before `SetMinimumWidth(300)`, so the minimum is 300 and is non-forced. Assuming no prior padding state or `OnTooltipCleared` handler side effect, and that `Short` is narrower than the minimum, `CalculateSize()` yields content width 300. Outer width is:

`300 + 2 × 10.24 + 0 = 320.48`

The two 10.24 values are the native horizontal border contributions. `GetWidth()` converts the native layout coordinate back to UI units. A padding value of 3.52 would produce 324; no 3.52 padding is set by the listed calls. The sequence itself does not establish a nonzero padding value: on a fresh tooltip it is zero. On a reused `GameTooltip`, earlier `SetPadding` state can persist because `SetOwner`/`ClearLines` do not clear it. An `OnTooltipCleared` script can also have side effects; its 12340 implementation was not available here.

**Not runtime-proven:** 320.48 is source-derived arithmetic, not a measured `GetWidth()` result. The exact shipped 12340 template/initialization scripts were unavailable, so this inspection cannot prove whether stock setup or a prior lifecycle handler sets/resets padding, nor categorically rule out a stock `>=324` result for the existing global tooltip. The inspected C++ path alone does not justify `>=324` as a stock oracle without evidence of at least 3.52 padding or wider content.

For the listed sequence, the tooltip has an owner and one line, so the tooltip's `ShowThis()` path proceeds: it restores alpha, clears fading, calculates size, then delegates to `CSimpleFrame::ShowThis()`. Ownerless or empty tooltips are hidden instead. This is source-derived flow, not runtime proof.

Evidence: W `WoW/Source/UIUtil/Tooltip_Scripts.cpp:83–95,115–119,187–190`; `WoW/Source/UIUtil/Tooltip.cpp:914–958,959–1084,1088–1098,1102–1143,1176–1194`; `Engine/Source/Frame/CScriptRegion_Scripts.cpp:250–273`; `Engine/Source/Frame/CSimpleFrame_Scripts.cpp:1013–1025`; `Engine/Source/Frame/CSimpleFrame.cpp:807–818,1892–1924`.

## 2. Cached UI source vs build 12340

**No exact match established.** The local UI source cache is labeled `wrath-3.4.3` at `/home/osso/.cache/wow-ui-sim/wow-ui-source/wrath-3.4.3/`; it is not identified as build 12340. The `wowdecomp` checkout reconstructs client code and explicitly says game data is not included. Thus the 3.4.3 files cannot certify shipped 12340 content or paths.

What the available 3.4.3 files actually show:

- `Interface/SharedXML/GameTooltipTemplate.xml` defines a virtual `GameTooltipTemplate` inheriting `GameTooltipCommonTemplate` and `TooltipBackdropTemplate`, with `$parentTextLeftN`/`$parentTextRightN` font-string names. It does not itself spell out a direct `<Backdrop>` element. `Interface/FrameXML/GameTooltip.xml` includes that shared XML and instantiates the global `GameTooltip` inheriting the template. `FrameXML_Wrath.toc` lists `SharedTooltipTemplates.xml` before `GameTooltip.xml` and `BuffFrame.xml` later. This is the cached 3.4.3 load graph only.
- `Interface/FrameXML/BuffFrame.lua` registers `UNIT_AURA` for `player` and `vehicle`; its event handler calls `BuffFrame_Update()` when the event unit equals `PlayerFrame.unit` (and handles additional roster/specialization events).
- `Interface/FrameXML/BuffFrame.xml` defines `AuraButtonTemplate`. Its `OnEnter` calls `GameTooltip:SetOwner(self, "ANCHOR_BOTTOMLEFT")`, sets frame level to `self:GetFrameLevel() + 2`, then calls `SetUnitAura(PlayerFrame.unit, self:GetID(), self.filter)`. `OnLeave` hides the tooltip. `BuffFrame.xml` lists `BuffFrame.lua` before the template.
- In this cached build, `GameTooltip.xml`'s instantiated tooltip has an `OnLoad` calling `GameTooltip_OnLoad`; its `OnHide` calls `GameTooltip_OnHide`. The shared tooltip template and backdrop inheritance are different layers; do not conflate them with target-build `Show()` behavior.

These observations identify loading and hover behavior in the 3.4.3 cache, not differences from 12340. No 12340-specific loading, hover, or `Show()` difference can be asserted from the available files. Runtime behavior and native/UI parity were not tested.

Cached-file evidence paths:
- `/home/osso/.cache/wow-ui-sim/wow-ui-source/wrath-3.4.3/Interface/SharedXML/GameTooltipTemplate.xml`
- `/home/osso/.cache/wow-ui-sim/wow-ui-source/wrath-3.4.3/Interface/FrameXML/GameTooltip.xml`
- `/home/osso/.cache/wow-ui-sim/wow-ui-source/wrath-3.4.3/Interface/FrameXML/FrameXML_Wrath.toc`
- `/home/osso/.cache/wow-ui-sim/wow-ui-source/wrath-3.4.3/Interface/FrameXML/BuffFrame.lua` and `BuffFrame.xml`

## Related

[Audit index and version limits](README.md) · [Prior 12340 UI source questions](ui-source-questions-12340.md)
