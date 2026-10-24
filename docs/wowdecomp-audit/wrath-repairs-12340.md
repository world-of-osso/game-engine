# Wrath repairs batch 1 — build 12340 Lua APIs

Source inspection date: 2026-10-23; evidence-boundary follow-up: 2026-10-24. Reference W is `/home/osso/Repos/wowdecomp` at `0e4e9fe0a4a9fa08027c5eed4b2c5e873739cfd3` (3.3.5a/build 12340 reconstruction). This page records native source and built-in Lua initialization only. The pinned reconstruction omits target-build FrameXML, so native method-table absence does not by itself establish final post-FrameXML lookups. No client, simulator, or server was run; no result transfers to another build.

## Q1 — S25: `pcall` nil-global error

After removing `_G.SetGuildRosterSelection`, the supplied closure calling `SetGuildRosterSelection(0)` produces exactly two `pcall` results in the bundled Lua source: `false` and an error string whose body, without its source/line prefix, is:

```text
attempt to call global 'SetGuildRosterSelection' (a nil value)
```

`luaG_typeerror` recovers the name from the `OP_GETGLOBAL` feeding the failing `OP_CALL`, using the single-quote `LUA_QS` format. `luaB_pcall` returns status plus all protected-call results, so the failure gives `(false, error_string)`. In the snippet as printed, the failing call is line 5 if compiled directly starting at line 1. The request provides no chunk name or compilation wrapper; FrameScript compilation accepts a caller-supplied chunk name, so the complete `chunk:line: body` string cannot be determined. Do not attribute an exact prefix. The bundled core identifies itself as Lua 5.1.1; runtime equivalence is unverified.

Evidence: W `Engine/Source/lua-5.1/src/lbaselib.c:352–359`; `ldebug.c:505–523,579–589,627–634`; `ldo.c:56–73,590–610`; `luaconf.h:201–202`; `lua.h:20`; `llex.c:160–161`; `Engine/Source/FrameScript/FrameScript.cpp:754–760,814–820,916–930,964–969`.

## Q2 — S35: `Frame:SetForbidden` / `Frame:IsForbidden`

Neither name is registered in the native `CSimpleFrame` method table or its inherited `CScriptRegion` table. Thus, at native method lookup, `frame.SetForbidden` and `frame.IsForbidden` are nil if no instance field or later Lua code has supplied them. The absence is not security-gated, so `secure` versus `forceinsecure` does not change the native registration result. With an unmodified nil lookup, a colon call fails as a nil-method call; there is no native argument conversion, return tuple, or forbidden-state transition to report. Because target-build FrameXML is absent, the final post-UI-script lookup in client 12340 is not established.

Evidence: W `Engine/Source/FrameScript/FrameScript_Object.cpp:31–49`; `Engine/Source/Frame/CScriptRegion_Scripts.cpp:734–767`; `CSimpleFrame_Scripts.cpp:1970–2074`; `Engine/Source/FrameScript/FrameScript_Scripts.cpp:299–304`.

## Q3 — S52: `GameTooltip:SetCustomWordWrapMinWidth`

The native `CGTooltip` method table contains `SetMinimumWidth` but no `SetCustomWordWrapMinWidth`; the pinned source therefore establishes no native binding for the requested setter. Its native method lookup would be nil unless an instance field or later Lua code supplies the name. Since target-build FrameXML is absent, the final `GameTooltip.SetCustomWordWrapMinWidth` lookup in client 12340 is not established. If the lookup remains nil, the supplied colon call errors at the setter with body `attempt to call method 'SetCustomWordWrapMinWidth' (a nil value)`, before `AddLine` or `Show`. No setter contract or numeric width/delta follows from the native source; the `GetWidth` binding returns one width value, but the requested sequence's value and sizing order are not established. The already answered default wrapped-row width is not repeated.

Evidence: W `WoW/Source/UIUtil/Tooltip_Scripts.cpp:2639–2716`; inherited lookup/registration `Engine/Source/FrameScript/FrameScript_Object.cpp:31–49`, `Engine/Source/Frame/CSimpleFrame_Scripts.cpp:1970–2074`; `GetWidth` `Engine/Source/Frame/CScriptRegion_Scripts.cpp:250–267`; nil-call diagnostic `Engine/Source/lua-5.1/src/ldo.c:270–275`, `ldebug.c:540–543,579–589`, `luaconf.h:201–202`.

## Q4 — S53: requested table-extension keys

After native `FrameScript_Initialize` opens the Lua table library and runs built-in `compat.lua`, `rawget(table, name)` returns one nil for each: `count`, `indexof`, `contains`, `isempty`, `removeunordered`, `removevalue`, `keys`, `values`. This describes initialized native state before any unavailable target-build UI script could mutate `table`. None is callable in that state, so no native argument signature or normal return tuple exists. In particular, `table.count({[1] = "array", [2] = false, named = "hash"})` returns no tuple; it errors with `attempt to call field 'count' (a nil value)` plus the evaluated chunk's location prefix. If called inside `pcall`, the result is `(false, error_string)`.

The registered native table library includes `removemulti`, not the requested keys. The compatibility chunk only aliases existing table functions into globals and does not extend `table`. The final post-FrameXML lookup in client 12340 is not established because those target scripts are unavailable.

Evidence: W `Engine/Source/lua-5.1/src/ltablib.c:321–341`; `Engine/Source/FrameScript/FrameScript.cpp:66–117,1152–1165`; `Engine/Source/lua-5.1/src/lbaselib.c:166–172`; `ldebug.c:532–533,579–589`.

## Related

[Audit index and proof limits](README.md) · [Earlier 12340 UI source questions](ui-source-questions-12340.md)
