#include "QuestConversation.h"

#include "Stage.h"

#include "../Character/QuestCatalog.h"
#include "../Character/QuestDialogue.h"
#include "../Character/QuestMarker.h"
#include "../IO/UI.h"
#include "../IO/UITypes/UINpcTalk.h"
#include "../Net/Packets/NpcInteractionPackets.h"
#include "../Net/Packets/QuestPackets.h"
#include "../Util/Misc.h"

#include <nlnx/nx.hpp>

#include <algorithm>
#include <map>
#include <optional>
#include <string>
#include <vector>

namespace ms
{
	namespace QuestConversation
	{
		namespace
		{
			struct Active
			{
				QuestDialogue dialogue;
				int16_t quest_id;
				int32_t npc_id;
			};

			std::optional<Active> active;
			// A follow-up quest to open once the current conversation ends.
			std::optional<std::pair<int16_t, int32_t>> next;

			// Menu choice for the NPC's normal speech; quest ids are positive.
			constexpr int32_t TALK_CHOICE = 0;

			Player& player()
			{
				return Stage::get().get_player();
			}

			int16_t state(int16_t quest_id)
			{
				return player().get_quests().state(quest_id);
			}

			bool can_start(const QuestInfo& quest)
			{
				auto& stats = player().get_stats();

				return quest_available(quest, stats.get_stat(MapleStat::Id::JOB), stats.get_stat(MapleStat::Id::LEVEL),
					[](int16_t id) { return state(id); });
			}

			bool items_met(const QuestInfo& quest)
			{
				const Inventory& inventory = player().get_inventory();

				return end_items_met(quest, [&](int32_t id) { return inventory.get_total_item_count(id); });
			}

			bool mobs_met(const QuestInfo& quest)
			{
				const auto& started = player().get_quests().started_quests();
				auto iter = started.find(quest.id);

				return end_mobs_met(quest, iter == started.end() ? std::string() : iter->second);
			}

			// Whether the server has normal speech for this NPC (info/speak).
			bool has_speech(int32_t npc_id)
			{
				return NxHelper::Npc::get_npc_node(npc_id)["info"]["speak"].size() > 0;
			}

			Texture list_header(int index)
			{
				return nl::nx::ui["UIWindow2.img"]["UtilDlgEx"]["list" + std::to_string(index)];
			}

			void send(QuestDialogue::Effect effect, int16_t quest_id, int32_t npc_id)
			{
				auto action = effect == QuestDialogue::Effect::START ? QuestActionPacket::START : QuestActionPacket::COMPLETE;

				if (effect != QuestDialogue::Effect::NONE)
					QuestActionPacket(action, quest_id, npc_id, player().get_position()).dispatch();
			}

			void show();

			void respond(int8_t response, int32_t selection)
			{
				if (!active)
					return;

				send(active->dialogue.respond(response, selection), active->quest_id, active->npc_id);
				show();
			}

			void show()
			{
				auto page = active ? active->dialogue.page() : std::nullopt;

				if (!page)
				{
					active.reset();

					if (auto followup = next)
					{
						next.reset();
						open(followup->first, followup->second);
					}

					return;
				}

				int8_t type = 0;
				int16_t style = 0;

				switch (page->kind)
				{
					case QuestPage::YESNO:
						type = 1;
						break;
					case QuestPage::SELECTION:
						type = 4;
						break;
					case QuestPage::TEXT:
						style = (page->previous ? 0x00FF : 0) | (page->next ? 0xFF00 : 0);
						break;
				}

				UI::get().emplace<UINpcTalk>();
				UI::get().enable();

				if (auto talk = UI::get().get_element<UINpcTalk>())
				{
					talk->change_text(active->npc_id, type, style, 0, page->text);
					talk->set_local_responder(respond);
				}
			}
		}

		void open(int16_t quest_id, int32_t npc_id)
		{
			const QuestInfo* quest = QuestCatalog::get().find(quest_id);

			if (!quest)
				return;

			std::optional<QuestDialogue::Mode> mode;

			if (state(quest_id) == 0 && quest->start_npc == npc_id && can_start(*quest))
				mode = QuestDialogue::Mode::OFFER;
			else if (state(quest_id) == 1 && quest->end_npc == npc_id)
				mode = QuestDialogue::Mode::COMPLETION;
			else if (state(quest_id) == 1 && quest->start_npc == npc_id)
				mode = QuestDialogue::Mode::REMINDER;

			if (!mode)
				return;

			bool completion_side = *mode != QuestDialogue::Mode::OFFER;
			QuestScript script = QuestCatalog::get().script(quest_id, completion_side);

			if (*mode == QuestDialogue::Mode::COMPLETION && !mobs_met(*quest))
			{
				// Say why it can't be finished yet (stop/mob, else stop/item).
				script.reminder = script.mob_reminder.empty() ? script.item_reminder : script.mob_reminder;
				mode = QuestDialogue::Mode::REMINDER;
			}
			else if (*mode == QuestDialogue::Mode::COMPLETION && !items_met(*quest))
			{
				// Say why it can't be finished yet (stop/item).
				script.reminder = script.item_reminder;
				mode = QuestDialogue::Mode::REMINDER;
			}
			else if ((*mode == QuestDialogue::Mode::OFFER && quest->start_script) ||
				(*mode == QuestDialogue::Mode::COMPLETION && quest->end_script))
			{
				// The server runs this step as a script and sends its dialogue.
				auto action = *mode == QuestDialogue::Mode::OFFER
					? QuestActionPacket::SCRIPT_START : QuestActionPacket::SCRIPT_END;
				QuestActionPacket(action, quest_id, npc_id, player().get_position()).dispatch();
				return;
			}

			active = Active { QuestDialogue(*mode, std::move(script)), quest_id, npc_id };
			send(active->dialogue.take_initial_effect(), quest_id, npc_id);
			show();
		}

		void continue_with(int16_t quest_id, int32_t npc_id)
		{
			if (active)
				next = std::make_pair(quest_id, npc_id);
			else
				open(quest_id, npc_id);
		}

		void talk_to_npc(int32_t npc_id, int32_t object_id)
		{
			// Same order as the original list: turn-ins, open quests, new quests.
			std::vector<int16_t> turn_in, in_progress, available;

			for (const auto& quest : QuestCatalog::get().quests())
			{
				int16_t current = state(quest.id);

				if (current == 1 && quest.end_npc == npc_id)
					turn_in.push_back(quest.id);
				else if (current == 1 && quest.start_npc == npc_id)
					in_progress.push_back(quest.id);
				else if (current == 0 && quest.start_npc == npc_id && can_start(quest))
					available.push_back(quest.id);
			}

			if (turn_in.empty() && in_progress.empty() && available.empty())
			{
				TalkToNPCPacket(object_id).dispatch();
				return;
			}

			// UtilDlgEx/list3 "Quests to turn in", list0 "Quests in progress",
			// list1 "Available quests", list2 "Misc." (normal speech).
			std::string text;
			std::map<int32_t, Texture> headers;

			for (auto [group, header] : { std::pair<const std::vector<int16_t>*, int>{ &turn_in, 3 }, { &in_progress, 0 }, { &available, 1 } })
			{
				for (size_t i = 0; i < group->size(); i++)
				{
					int16_t id = (*group)[i];

					if (i == 0)
						headers[id] = list_header(header);

					text += "#L" + std::to_string(id) + "#" + QuestCatalog::get().find(id)->name + "#l";
				}
			}

			if (has_speech(npc_id))
			{
				headers[TALK_CHOICE] = list_header(2);
				std::string name = nl::nx::string["Npc.img"][std::to_string(npc_id)]["name"];
				text += "#L" + std::to_string(TALK_CHOICE) + "#Talk to " + name + "#l";
			}

			active.reset();
			next.reset();

			UI::get().emplace<UINpcTalk>();
			UI::get().enable();

			if (auto talk = UI::get().get_element<UINpcTalk>())
			{
				talk->change_text(npc_id, 4, 0, 0, text, headers);
				talk->set_local_responder([npc_id, object_id](int8_t response, int32_t selection)
				{
					if (response != 1)
						return;

					if (selection == TALK_CHOICE)
						TalkToNPCPacket(object_id).dispatch();
					else
						open(static_cast<int16_t>(selection), npc_id);
				});
			}
		}
	}
}
