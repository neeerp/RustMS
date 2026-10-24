#pragma once

#include "PartyQuests.h"
#include "QuestDialogue.h"
#include "QuestInfo.h"

#include <cstdint>
#include <istream>
#include <vector>

#include <nlnx/node.hpp>

namespace ms
{
	class QuestCatalog
	{
	public:
		// Read the quests listed in `supported` (one id per line, `#`
		// comments), and every party quest, from a Quest.nx root node.
		QuestCatalog(nl::node quest_root, std::istream& supported);

		// The quests in supported_quests.txt, which the build copies next to
		// the executable, from the loaded Quest.nx.
		static const QuestCatalog& get();

		const QuestInfo* find(int16_t id) const;
		// Sorted by id.
		const std::vector<QuestInfo>& quests() const;
		// Sorted by id. They are never offered, so they aren't in quests().
		const std::vector<PartyQuest>& party_quests() const;

		// Say.img/<id>/0 (offer) or /1 (completion and reminders).
		QuestScript script(int16_t id, bool completion) const;

		// A title (medal) quest, in QuestInfo.img's area 51: v83 lists it in
		// the title window, not the quest log or the quest helper
		// (FUN_00a26faa, FUN_00721126).
		bool is_title(int16_t id) const;

		static constexpr int16_t TITLE_AREA = 51;

	private:
		nl::node root;
		std::vector<QuestInfo> supported;
		std::vector<PartyQuest> party;
	};
}
