#pragma once

#include "QuestAlertRules.h"

#include <cstdint>
#include <functional>
#include <vector>

namespace ms
{
	struct QuestInfo;

	// The client side of the v83 quest flow. The client lists an NPC's quests,
	// renders their Say.img conversations, and sends QUEST_ACTION; the server
	// only validates and applies the result.
	namespace QuestConversation
	{
		// The player clicked an NPC. Shows its quest menu, or asks the server
		// for the NPC's normal speech when it has no quests for the player.
		void talk_to_npc(int32_t npc_id, int32_t object_id);

		// Open a quest's conversation with an NPC: its offer, completion, or
		// reminder, depending on the quest's state.
		void open(int16_t quest_id, int32_t npc_id);

		// The server started or completed a quest (0xD3 mode 8): the NPC
		// says its yes pages, then `next_quest`, if any, opens once they
		// have all been read (FUN_00959a73).
		void confirmed(int16_t quest_id, int32_t npc_id, int16_t next_quest);

		// Whether the player can start a quest now, and complete a started one with an NPC.
		bool startable(const QuestInfo& quest);
		bool completable(const QuestInfo& quest, int32_t npc_id);

		// The quest alert was clicked: Maple Administrator lists its quests (FUN_0095d260), and `chosen` gets the one
		// picked.
		void alert_menu(const std::vector<quest_alert::Entry>& entries, std::function<void(int16_t)> chosen);
	}
}
