# Toy Box

Retail Mainline defines CollectionsJournal geometry (703×606), tab order (Mounts, Pets, Toy Box, Heirlooms, Appearances, Warband Scenes), and ToyBox's 18 entries/page, 3 columns × 6 rows. Forever changes art only.

- Both skins show each toy name to its icon's right, including greyed uncollected names and desaturated uncollected icons. Empty catalog names display ItemID, matching Retail ToySpellButton_UpdateButton; search and a graphical collection progress bar remain present in both skins.
- Full ToyCollectionUpdate replaces catalog/ownership/favourites. Initial ownership is not new; subsequent learning glows and navigates to the filtered page containing that toy.
- Collected/uncollected/usable, source, expansion and case-insensitive name search filter the real catalog. Favourites sort first, then name and ItemID. Empty filters show an empty result. Page changes clamp.
- Left click sends UseToy only for learned, available entries. Right click on learned toys offers Set/Remove Favorite. Server results display refusals; successful Use acknowledges cast start, not completion. Existing CastFailed/SpellGo remain authoritative.
- SpellCooldownUpdate maps through toy spell IDs; GCD is not a toy cooldown. The journal and action bars show remaining cooldown.
- Drag copies ToyAction to any existing action bar slot. Retail PickupToyBoxItem is persisted as ActionRef::Item(ItemID), using existing SetActionButton and server persistence. A catalog toy slot activates through the same UseToy path, not UseItem or bag possession. No protocol/server patch is needed.
- Hover displays toy identity, source, ownership and unavailable reason. Collections micro-menu opens/closes the journal.

- Replicated UnitScale applies aura61 additive scale to the unit root, preserving each native visual's own scale and restoring1 on removal. Both skins share this world presentation.

- Replicated FeatherFall caps only downward player prediction at7yd/s while aura105 is active; upward jumps retain their impulse and removal restores ordinary gravity. Reference: TrinityCore MovementUtil.cpp terminalSafefallVelocity and a352b1fa HandleAuraFeatherFall.

- Aura293 SpellSet extends existing aura332 AuraView.overrides consumption: first six main slots use OverrideSpellData order, zero entries and remaining main slots are empty, other bars stay unchanged. Newest set wins; removal restores stored actions without rebinding. Existing main-bar art is retained; dedicated Retail OverrideActionBar chrome is not added here.

## Reference and explicit boundaries

Local source: `~/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/Blizzard_Collections/Mainline/Blizzard_Collections.xml`, `Blizzard_Collections.lua`, `Blizzard_ToyBox.xml`, `Blizzard_ToyBox.lua`, `Blizzard_CollectionTemplates.xml`.

Other journal features are not implemented here. Their tabs remain visible but disabled, without invented content. Retail does not generally disable missing feature tabs: Collections.lua hides Heirlooms specifically during Timerunning. Disabled unimplemented native tabs are a documented client limitation, not a Retail parity claim. Model-scene fanfare unwrapping is not implemented; new-toy glow and page-to-new-toy are required here. Toy spell effects and eligibility come from game-server master; this client does not expand effect coverage.
