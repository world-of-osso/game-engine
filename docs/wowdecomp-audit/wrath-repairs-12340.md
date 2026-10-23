# Wrath repairs batch 1 — build 12340 Lua APIs

Source inspection date: 2026-10-23. Reference W is `/home/osso/Repos/wowdecomp` at `0e4e9fe0a4a9fa08027c5eed4b2c5e873739cfd3` (3.3.5a/build 12340 reconstruction). This page records source-derived argument, return, lookup, and error behavior only. No client, simulator, or server was run; no result transfers to another build. The pinned reconstruction does not include target-build FrameXML scripts, so later Lua monkeypatches are outside the proof.

## Q1 — S25: `pcall` nil-global error

After removing `_G.SetGuildRosterSelection`, the supplied closure calling `SetGuildRosterSelection(0)` is inferred to produce exactly two `pcall` results: `false` and an error string whose body, without source/line prefix, is:

```text
attempt to call global 'SetGuildRosterSelection' (a nil value)
```

`luaG_typeerror` recovers the name from the `OP_GETGLOBAL` feeding the failing `OP_CALL`, using the single-quote `LUA_QS` format. `luaB_pcall` returns status plus all protected-call results, so the failing closure gives `(false, error_string)`. Prefix is the evaluated chunk's `luaO_chunkid` and current source line; the request clarification leaves the simulator-selected chunk name unspecified. The bundled core labels itself Lua 5.1.1 and follows the stock Lua 5.1 name-recovery path here; no WoW-specific difference is visible in source. Runtime equivalence is unknown.

Evidence: W `Engine/Source/lua-5.1/src/lbaselib.c:352–359`; `ldebug.c:505–531,579–589,627–634`; `luaconf.h:201–202`; `lua.h:21–24`; `llex.c:160–161`.

## Q2 — S35: `Frame:SetForbidden` / `Frame:IsForbidden`

Both `frame.SetForbidden` and `frame.IsForbidden` resolve to one nil value on a normal 12340 Frame. Secure versus `forceinsecure` callers do not change the result: `__index` uses the registered method table, and `forceinsecure` changes taint, not that table. The inherited `CScriptRegion` and `CSimpleFrame` method registrations contain neither name. No callable signature, conversion, return tuple, or forbidden-state transition exists in this source snapshot. A colon call fails as a nil method call.

Evidence: W `Engine/Source/FrameScript/FrameScript_Object.cpp:31–49`; `Engine/Source/Frame/CScriptRegion_Scripts.cpp:734–767`; `CSimpleFrame_Scripts.cpp:1981–2074`; `Engine/Source/FrameScript/FrameScript_Scripts.cpp:299–304`.

## Q3 — S52: `GameTooltip:SetCustomWordWrapMinWidth`

The `CGTooltip` method lookup is nil. Its method table contains `SetMinimumWidth`, but not `SetCustomWordWrapMinWidth`. Consequently the supplied sequence raises `attempt to call method 'SetCustomWordWrapMinWidth' (a nil value)` at the setter call, before `AddLine` and `Show`; there is no resulting width, setter/baseline delta, or update order for this absent method. The already answered default wrapped-row width is not repeated.

Evidence: W `WoW/Source/UIUtil/Tooltip_Scripts.cpp:2639–2716`; inherited lookup/registration `Engine/Source/FrameScript/FrameScript_Object.cpp:31–49`, `Engine/Source/Frame/CSimpleFrame_Scripts.cpp:1981–2074`.

## Q4 — S53: requested table-extension keys

At completion of native `FrameScript_Initialize` (the Lua table library plus built-in `compat.lua`), `rawget(table, name)` returns one nil for each: `count`, `indexof`, `contains`, `isempty`, `removeunordered`, `removevalue`, `keys`, `values`. None is callable, so no argument signature or normal return tuple exists. In particular, `table.count({[1] = "array", [2] = false, named = "hash"})` returns no tuple; it errors with `attempt to call field 'count' (a nil value)` plus the evaluated chunk's location prefix. If called inside `pcall`, the result is `(false, error_string)`.

The registered native table library includes `removemulti`, not the requested keys. The compatibility chunk only aliases existing table functions into globals and does not extend `table`. Later target-build Lua could mutate `table`; the absent FrameXML source leaves that possibility unverified.

Evidence: W `Engine/Source/lua-5.1/src/ltablib.c:321–341`; `Engine/Source/FrameScript/FrameScript.cpp:66–117,1152–1165`; `Engine/Source/lua-5.1/src/ldebug.c:505–531,579–589`.

## Related

[Audit index and proof limits](README.md) · [Earlier 12340 UI source questions](ui-source-questions-12340.md)
