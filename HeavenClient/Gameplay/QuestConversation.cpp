#include "QuestConversation.h"

#include "Stage.h"

#include "../Character/QuestCatalog.h"
#include "../Character/QuestDialogue.h"
#include "../Character/QuestMarker.h"
#include "../Data/ItemData.h"
#include "../IO/UI.h"
#include "../IO/UITypes/UINpcTalk.h"
#include "../Net/Packets/NpcInteractionPackets.h"
#include "../Net/Packets/QuestPackets.h"
#include "../Util/Misc.h"
#include "../Util/NxFiles.h"

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
				// Opens once the dialogue has been read to the end.
				int16_t next_quest = 0;
			};

			std::optional<Active> active;

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

			QuestStop completion_stop(const QuestInfo& quest, int32_t npc_id)
			{
				const Inventory& inventory = player().get_inventory();
				const auto& started = player().get_quests().started_quests();
				auto progress = started.find(quest.id);

				return ms::completion_stop(quest, npc_id, [](int16_t id) { return state(id); },
					[&](int32_t id) { return inventory.get_total_item_count(id); },
					progress == started.end() ? std::string() : progress->second);
			}

			// Whether a quest item given at the start is no longer held, worn
			// or not (FUN_00718b58).
			bool lost_start_items(const QuestInfo& quest)
			{
				const Inventory& inventory = player().get_inventory();

				return std::any_of(quest.start_items.begin(), quest.start_items.end(), [&](const auto& item)
				{
					int32_t held = inventory.get_total_item_count(item.first) +
						inventory.count_slots(InventoryType::Id::EQUIPPED, item.first);

					return ItemData::get(item.first).is_questitem() && held < item.second;
				});
			}

			// What the NPC says when a quest can't be completed with it, or
			// yet: the NPC's own d0 line when the quest has nothing to say.
			std::vector<std::string> stop_pages(const QuestScript& script, QuestStop stop, bool lost, int32_t npc_id)
			{
				if (lost && !script.lost.empty())
					return script.lost;

				std::vector<std::string> pages = script.stop(stop_reason(stop));

				if (pages.empty())
				{
					std::string line = nl::nx::string["Npc.img"][std::to_string(npc_id)]["d0"];

					if (!line.empty())
						pages.push_back(line);
				}

				return pages;
			}

			// Whether the server has normal speech for this NPC (info/speak).
			bool has_speech(int32_t npc_id)
			{
				return NxHelper::Npc::get_npc_node(npc_id)["info"]["speak"].size() > 0;
			}

			Texture list_header(int index)
			{
				return NxFiles::ui_classic["UIWindow.img"]["UtilDlgEx"]["list" + std::to_string(index)];
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

			void begin(Active conversation)
			{
				active = std::move(conversation);
				send(active->dialogue.take_initial_effect(), active->quest_id, active->npc_id);
				show();
			}

			void show()
			{
				auto page = active ? active->dialogue.page() : std::nullopt;

				if (!page)
				{
					if (active && active->dialogue.read() && active->next_quest != 0)
					{
						int16_t next_quest = active->next_quest;
						int32_t npc_id = active->npc_id;
						active.reset();
						open(next_quest, npc_id);
					}
					else
					{
						active.reset();
					}

					return;
				}

				int8_t type = 0;
				int16_t style = 0;

				switch (page->kind)
				{
					case QuestPage::YESNO:
						// A quest's offer ends with ACCEPT and DECLINE.
						type = 12;
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
					talk->change_text(active->npc_id, type, style, 0, page->text, {}, UINpcTalk::Menu::LIST);
					talk->set_local_responder(respond);
				}
			}
		}

		void open(int16_t quest_id, int32_t npc_id)
		{
			const QuestInfo* quest = QuestCatalog::get().find(quest_id);

			if (!quest)
				return;

			if (state(quest_id) == 0)
			{
				if (quest->start_npc != npc_id || !can_start(*quest))
					return;

				if (quest->start_script)
				{
					// The server runs this step as a script and sends its dialogue.
					QuestActionPacket(QuestActionPacket::SCRIPT_START, quest_id, npc_id, player().get_position()).dispatch();
					return;
				}

				begin({ QuestDialogue(QuestDialogue::Mode::OFFER, QuestCatalog::get().script(quest_id, false)), quest_id, npc_id });
				return;
			}

			if (state(quest_id) != 1)
				return;

			// v83 checks Check.img/<id>/1 before saying anything (FUN_00716fe1).
			QuestScript script = QuestCatalog::get().script(quest_id, true);
			QuestStop stop = completion_stop(*quest, npc_id);

			if (stop != QuestStop::NONE)
			{
				bool lost = stop == QuestStop::NPC && lost_start_items(*quest);
				begin({ QuestDialogue::reading(stop_pages(script, stop, lost, npc_id)), quest_id, npc_id });
				return;
			}

			if (quest->end_script)
			{
				QuestActionPacket(QuestActionPacket::SCRIPT_END, quest_id, npc_id, player().get_position()).dispatch();
				return;
			}

			auto& stats = player().get_stats();
			std::string rewards = quest->rewards.summary(stats.get_stat(MapleStat::Id::JOB), stats.get_female(),
				quest->next_quest != 0);

			begin({ QuestDialogue(QuestDialogue::Mode::COMPLETION, std::move(script), std::move(rewards)), quest_id, npc_id });
		}

		void confirmed(int16_t quest_id, int32_t npc_id, int16_t next_quest)
		{
			// The quest's record arrives first, so a started quest is in
			// progress and a completed one is not.
			bool completed = state(quest_id) != 1;
			QuestScript script = QuestCatalog::get().script(quest_id, completed);

			if (script.yes.empty())
			{
				if (next_quest != 0)
					open(next_quest, npc_id);

				return;
			}

			begin({ QuestDialogue::reading(std::move(script.yes)), quest_id, npc_id, next_quest });
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

			// UtilDlgEx/list3 "QUEST THAT CAN BE COMPLETED", list0 "QUEST IN
			// PROGRESS", list1 "QUEST AVAILABLE", list2 "ETC" (normal speech,
			// worded as in period footage).
			std::string text;
			std::map<int32_t, Texture> headers;

			for (auto [group, header] : { std::pair<const std::vector<int16_t>*, int>{ &turn_in, 3 }, { &in_progress, 0 }, { &available, 1 } })
			{
				for (size_t i = 0; i < group->size(); i++)
				{
					int16_t id = (*group)[i];

					if (i == 0)
						headers[id] = list_header(header);

					text += " #L" + std::to_string(id) + "# " + QuestCatalog::get().find(id)->name + "#l";
				}
			}

			if (has_speech(npc_id))
			{
				headers[TALK_CHOICE] = list_header(2);
				text += " #L" + std::to_string(TALK_CHOICE) + "# I have something to tell you.#l";
			}

			active.reset();

			UI::get().emplace<UINpcTalk>();
			UI::get().enable();

			if (auto talk = UI::get().get_element<UINpcTalk>())
			{
				talk->change_text(npc_id, 4, 0, 0, text, headers, UINpcTalk::Menu::LIST);
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
