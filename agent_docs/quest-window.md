# Quest window

The client uses the original two-panel layout from `UI.nx/UIWindow2.img/Quest`:
the 295×396 list and the 296×396 information panel. The details background links
into `UIWindow3.img/Quest/quest_info`. `Texture` resolves these asset links;
no extracted artwork is tracked in Git.

The layout reference is Nexon's [2013 quest UI preview](https://archive.maplestory.nexon.com/News/Update/303).
The local assets provide the frames, tabs, search field, empty notices, quest
icons, completed-count header, portrait panel, reward panel/headings, scrollbars,
and action buttons. Solid category/selection bars use the reference's colors.

Available quests share `QuestLog::supported_rules()` and eligibility rules with
NPC markers, so unsupported asset quests are not advertised. The level filter
can display out-of-range quests without enabling their acceptance. Job and
prerequisite checks still apply. Started and completed records are read from
the server's quest ledger.

Select a list entry to show its NPC portrait, level range, destination, original
`QuestInfo.img/<id>/<state>` description, completion date, and EXP reward. Search
is case insensitive. The list and detail pane scroll independently, and the
category can be collapsed. Filters switch between all/current locations and
all/current levels. Current-location filtering uses NPCs actually present in
the field. The complete window is kept within the viewport when opened/resized.

Find NPC, or double-clicking the portrait, opens the world map with the destination
marked. NPC locations are resolved by traversing the active beginner portal graph
from `10000`, avoiding the disconnected legacy map variants. Accept/Finish opens
the existing server-validated NPC conversation when that NPC is present and its
quest marker permits the action; it does not bypass dialogue requirements.

The Recommended tab, Quest Helper, and Forfeit use disabled states: recommendations,
tracking overlays, and abandonment do not yet have implementations. Displayed reward details currently
cover the EXP rewards of the supported quests. Adding item/meso rewards requires
matching server support and reward UI rows.

Validation: build the native client and run CTest; marker tests include available
quest filtering across jobs, prerequisites, and level modes. Use a private Xvfb
client connected to the isolated harness stack for rendering/input checks. Seeded
quest records in that disposable database can exercise long lists and scrolling
without altering the development characters. Check all three tabs, search,
empty results, dragging, collapse/expand, detail closing/reopening, and Find NPC.
