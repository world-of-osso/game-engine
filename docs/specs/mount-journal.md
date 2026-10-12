# Mount Journal

Approved2026-10-11. Retail Mainline Collections→Mounts in both skins; Forever modifies art only.

Provide the Retail703×606 Collections shell/tab bar,46px scrolling mount rows, search, collected/not-collected, type and source filters; total owned count; selected name, authentic source/description and rotating creature-display preview; summon/dismount, account favourites and Summon Random Favorite Mount. Collection/favourites persist account-wide across characters and restarts.

Server authority uses authentic Retail12.1.0.69933 Mount/MountXDisplay/MountCapability/MountTypeXCapability and existing read-only world.db item/vendor/quest sources. Mount IDs are Mount.db2 IDs, never creature display IDs. Item learning consumes one item atomically with account ownership; duplicate learning preserves the physical item. Normal spell-learning results migrate authentic source spells to account ownership. Existing mount spell casting/riding/area/flight rules remain authoritative. Unowned mounts cannot summon/favourite; random summon uses only owned usable favourites. Dismount cancels the mount aura.

References: local Blizzard_Collections/Mainline/Blizzard_MountCollection.lua/.xml; server mount logic follows TrinityCore a352b1fa. Local CASC only for missing assets; no fabricated IDs/assets. Completion: behavioural tests, live privateUDP5901 fb_* account learning/summon/favourite/random summon/relogin in both skins; inspected PNGs in /syncthing/AgentShared/2026-10-11/mount-journal/. No canonical world.db mutation.
