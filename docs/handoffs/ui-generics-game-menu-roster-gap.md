# Game Menu roster qualification — outside Step 5

Verified: 2026-10-23. Retail UI cache: `12.1.5.70448`, rooted at `/home/osso/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/` (`R` below). No product controls are changed by this qualification. [Existing menu contract and deferred capabilities](../specs/game-menu-settings.md).

## Exact source order and conditions

`R/Blizzard_GameMenu/Shared/GameMenuFrame.lua:113–218` initializes, in order:

| Section | Retail order and condition | Captured/native qualification |
|---|---|---|
| Optional preface | External Event only with `C_ExternalEventURL.HasURL()`, then section break (`125–134`). | No external-event capability/condition is represented by this fixture; omission is unqualified, not proven Retail parity. |
| Options/shop | Options (`139`); Shop if StorePublic or CatalogShop is enabled (`148–155`); section break (`157`). | Options shown. Shop is an existing explicitly deferred capability, not a Step 5 addition; its source visibility condition is known, not exercised here. |
| Middle | Rewards for an active account storefront (`163–173`); AddOns via `ShouldShowAddOns()` (`175–178`); What's New via `ShouldShowSplashScreen()` (`180–182`); Edit Mode via `CanEnterEditMode()` (`184–193`); Ratings via its shown flag (`195–198`); Support (`200`); Macros unless the disabling game rule is active (`202–210`). | Screenshot shows AddOns → Support → Macros. Relative order is correct for that subset. The fixture does not establish every omitted/present conditional state. |
| Exit | Section break (`212`), Log Out (`215`), Exit Game (`216`). Kiosk/visible CAMP/PLUNDERSTORM_LEAVE/QUIT only **disable** these buttons (`214`), not remove them. | Offline fixture omits Log Out through native `logged_in=false`; no equivalent Retail hide condition exists in this initializer. **Filed gap**, not a source-qualified omission. Actual character-session native menus include it. |
| Return | `AddCloseButton(RETURN_TO_GAME)` (`218`), which adds a section break before Return to Game. | Present after Exit Game with the expected section gap. |

`R/Blizzard_SharedXML/Shared/Frame/MainMenuFrameTemplates.lua:25–31` preserves insertion order with `layoutIndex` and applies pending section spacing to the next button. `69–71` defines a section as20px; `73–79` adds that section before the close button. The native three20px spacers match the source section positions for its rendered subset: after Options (Shop absent), after Macros, and before Return. No inter-button gap is added within a section. `R/Blizzard_SharedXML/Mainline/Frame/MainMenuFrameTemplates.xml:11–15,22–33` supplies200×36 buttons, padding48/34/28/28, spacing0, width260; the native constants use those values.

## Existing conditions versus remaining gaps

- Store/Splash/Ratings are already deferred in the [menu spec](../specs/game-menu-settings.md#known-gaps-current-cycle). Splash's explicit absence condition is `R/Blizzard_SharedXML/GameRulesUtil.lua:70–76`: standard mode, non-null/viewable `C_SplashScreen`, and not newly boosted. Ratings is enabled for koKR by `R/Blizzard_GameMenu/Shared/Localization.lua:9–13`, not enUS. These captures do not demonstrate supported Shop/Splash behavior.
- Native `game_menu_main.rs::menu_buttons` uses only `logged_in` to include Edit Mode and Log Out. The fixture explicitly sets it false; normal `game_menu.rs:88–89` derives it from an active character session. This explains the six-button screenshot but is **not** a Retail Logout visibility condition.
- Edit Mode's source predicate additionally checks the disabling rule, NPE restriction, account settings and blocking frames (`R/Blizzard_EditMode/Shared/EditModeManager.lua:1636–1655`). The native session boolean does not establish those conditions. Conditional eligibility remains unqualified.
- AddOns and Macros are unconditional in the native list; Retail gates them (`GameMenuFrame.lua:175–178,202–210`; AddOns details in `GameRulesUtil.lua:58–68`). External Event and Rewards have no native conditional representation. These are roster/eligibility gaps to track outside Step 5, not fixes authorized by lifecycle work.

The accepted Escape milestone is an ordering/metadata proof, **not complete Game Menu roster parity**. Its original blank after-images show menu removal only; retained unmounted pending state was JSON-only. The debug fixture now adds a visible diagnostic marker driven by the real pending/menu state; final pixel acceptance must use recaptured marker images, not reinterpret the old blank frames.

## Source hashes

- `GameMenuFrame.lua`: `271ac99be5a3e44fe5cc9cb6d9953d06ef02f6d33028c28f73974abc3f43bcb0`
- `MainMenuFrameTemplates.lua`: `0ccb5cf25f0c81b3d3f3e5fd169265c6162cd69b74937109bee471761f0cee11`
- `MainMenuFrameTemplates.xml`: `20b2893228591557d8a249d8ab524c92fef6238d6e60669e1941c841992c6207`
