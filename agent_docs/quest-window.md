# Quest window

`UIQuestLog` draws the v83 quest log from `UIClassic.nx`
`UIWindow.img/Quest`: the 245×396 list (`backgrnd`) and, while a quest is
selected, the 305×396 detail panel (`backgrnd2`) beside it. No extracted
artwork is tracked in Git. The layout was measured from 2009 footage
(Available, In Progress, and Complete tabs, with and without details).

The list has three tabs (`Basic.img/Tab2`, drawn by `ClassicTabs`): Available,
In Progress, and Complete. Quests are grouped under a light blue bar per
`QuestInfo.img/<id>/area`, labelled with the area's name and count. The
`Basic.img` minus and plus boxes fold and unfold a group. The names come from
footage (Maple Island, Victoria Island, Job, World Tour, Event, "Elnath Mt. +
Aquaroad"); the rest (Knights of Cygnus, Ludus Lake, Minar Forest, "Mu Lung +
Nihal Desert", Masteria, Temple of Time, Malaysia) are guesses from each area's
quests. Rows show the quest's own name with an icon:

- `icon0`, the exclamation mark, for available quests
- the animated `icon2` arrow for open quests
- the pulsing `icon3` ring for open quests whose objectives are all met
- `icon4` for completed quests (footage doesn't show this tab's icon)

Names run up to the scroll bar. The selected row has a dark blue bar and
white text that ends in "..". Both panels scroll with
`Basic.img/VScr4` (`Slider::Type::CLASSIC`) and the mouse wheel. An empty tab
shows its `notice` image.

Available quests share `QuestLog::supported_rules()` (built from the client's
`QuestCatalog`, see [Quests](./quests.md)) and eligibility rules with NPC
markers. That keeps unsupported asset quests off the list. Started and
completed records come from the server's quest ledger.

The detail panel shows:

- the NPC's portrait (the giver on Available, the taker otherwise)
- the bold quest name
- "Available to all" or the allowed jobs
- "Over Level N" or "Under Level N"
- the original `QuestInfo.img/<id>/<tab>` text

The text keeps its `#b`/`#k` colours and `#p`/`#m`/`#t`/`#o` names. `#c` and
`#a` show live counts, and `#i`/`#v` item icons sit inline. `#k` returns to the
text's own colour, black here. `Basic.img/BtHide` closes the panel. The buttons
along the bottom depend on the tab:

- Available has FIND NPC (`BtMarkNpc`), which opens the world map with the
  giver's map marked. NPC maps come from walking the portal graph out from
  the Explorer start (`0`) and each continent's towns, which skips the disconnected
  legacy map variants. When no world map lists the NPC's map, it says so
  instead, as v83 does.
- In Progress has QUEST HELPER (`BtAlert`), which tracks or untracks the
  quest in the Quest Helper, and FORFEIT (`BtGiveup`). FORFEIT asks "Are you
  sure you want to quit?", then untracks the quest and sends the forfeit, as
  v83 does. A quest the quest alert offers is offered again once it is gone.

The window has no Accept/Finish: v83 started and finished quests through NPC
dialogue only. It also has no search, filters, Recommended tab, reward panel,
or completion date, since the 2009 window had none. Several assets are left
unused: `backgrnd4` (a detail panel with a reward box), `notice`'s region
headings, `icon5` ("new"), `Gauge`, `prob`, `reward`, `select`, and `summary`.
They may belong to later v83 revisions.

Validation: build the native client and run CTest. The marker tests include
available-quest filtering across jobs, prerequisites, and levels. For rendering
and input checks, connect a private Xvfb client to the isolated harness stack.
Seeded quest records in that disposable database can exercise long lists and
scrolling without touching the development characters. A character inserted
straight into the database has no keybindings, so seed `keybindings` too, or Q
won't open the window. Check:

- all three tabs and empty tabs
- dragging the window
- folding and unfolding groups
- scrolling both panels
- hiding and reopening the details
- FIND NPC and QUEST HELPER
