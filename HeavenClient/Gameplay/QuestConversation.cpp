#include "QuestConversation.h"

#include "NpcScripts.h"
#include "Stage.h"

#include "../Character/QuestCatalog.h"
#include "../Character/QuestDialogue.h"
#include "../Character/QuestMarker.h"
#include "../Data/ItemData.h"
#include "../IO/UI.h"
#include "../IO/UITypes/UINpcTalk.h"
#include "../IO/UITypes/UINotice.h"
#include "../Net/Packets/NpcInteractionPackets.h"
#include "../Net/Packets/QuestPackets.h"
#include "../Util/Misc.h"
#include "../Util/NxFiles.h"

#include <nlnx/nx.hpp>

#include <algorithm>
#include <ctime>
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

			// Menu choice for the NPC's script; quest ids are positive.
			constexpr int32_t TALK_CHOICE = 0;

			// Maple Administrator, who speaks for the quest alert, and what he says first (StringPool 0x104b).
			constexpr int32_t ALERT_NPC = 9010000;
			constexpr const char* ALERT_GREETING = "Someone in MapleStory would like to send you a message.";
			// v83's notices for a pet requirement at completion (0xdb2, 0xdb3).
			constexpr const char* NO_PET = "Unable to complete the quest\\ndue to a lack of pet.\\nPlease try again "
				"after summoning a pet.";
			constexpr const char* WRONG_PET = "Unable to complete the quest\\nwith the current pet you have "
				"summoned.\\nPlease try again with a pet\\nthat meets the quest requirement.";

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
					[](int16_t id) { return state(id); },
					[](const QuestInfo& info) { return player().quest_start_met(info); });
			}

			QuestStop completion_stop(const QuestInfo& quest, int32_t npc_id)
			{
				const Inventory& inventory = player().get_inventory();
				const auto& started = player().get_quests().started_quests();
				auto progress = started.find(quest.id);
				auto record = started.find(quest.end_info_record);

				QuestStop stop = ms::completion_stop(quest, npc_id, [](int16_t id) { return state(id); },
					[&](int32_t id) { return inventory.get_total_item_count(id); },
					progress == started.end() ? std::string() : progress->second, inventory.get_meso(),
					player().first_pet(), record == started.end() ? nullptr : &record->second);

				if (stop == QuestStop::NONE &&
					!buffs_met(quest.end_buff, quest.end_except_buff, [](int32_t id) { return player().has_item_buff(id); }))
					return QuestStop::BUFF;

				const MonsterBook& book = player().get_monsterbook();

				if (stop == QuestStop::NONE &&
					!book_met(quest, book.count_cards(false) + book.count_cards(true), [&](int32_t card) { return book.get_level(card); }))
					return QuestStop::ITEM;

				if (stop == QuestStop::NONE && !party_rank_met(quest, player().get_quests().rank_count("S")))
					return QuestStop::PARTY_RANK;

				return stop;
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

			// The NPC's script (Npc/<id>.img/info/script/<n>/script, then
			// what 0x107 added): the newest whose start and end dates
			// include today (FUN_006de2d3, FUN_006dedf7). The server runs it
			// when the NPC is talked to.
			std::string npc_script(int32_t npc_id)
			{
				std::time_t now = std::time(nullptr);
				std::tm local = *std::localtime(&now);
				int32_t today = ((local.tm_year + 1900) * 100 + local.tm_mon + 1) * 100 + local.tm_mday;
				nl::node scripts = nl::nx::npc[string_format::extend_id(npc_id, 7) + ".img"]["info"]["script"];
				std::vector<npc_scripts::Window> windows;

				for (int n = 0; nl::node entry = scripts[std::to_string(n)]; n++)
					windows.push_back({ entry["script"],
						static_cast<int32_t>(entry["start"].get_integer(19000101)),
						static_cast<int32_t>(entry["end"].get_integer(20790101)) });

				auto received = npc_scripts::received().find(npc_id);

				if (received != npc_scripts::received().end())
					windows.insert(windows.end(), received->second.begin(), received->second.end());

				return npc_scripts::active(windows, today);
			}

			// The NPC's own line, from String/Npc.img: d0 greets, d1 is for
			// when all of its quests are done.
			std::string npc_line(int32_t npc_id, const char* line)
			{
				return nl::nx::string["Npc.img"][std::to_string(npc_id)][line];
			}

			Texture list_header(int index)
			{
				return NxFiles::ui_classic["UIWindow.img"]["UtilDlgEx"]["list" + std::to_string(index)];
			}

			void send(QuestDialogue::Effect effect, int16_t quest_id, int32_t npc_id)
			{
				auto action = effect == QuestDialogue::Effect::START ? QuestActionPacket::START : QuestActionPacket::COMPLETE;
				const QuestInfo* quest = QuestCatalog::get().find(quest_id);
				std::optional<Point<int16_t>> position;

				if (!quest || !(quest->auto_start || quest->auto_complete))
					position = player().get_position();

				if (effect != QuestDialogue::Effect::NONE)
					QuestActionPacket(action, quest_id, npc_id, position, active->dialogue.choice()).dispatch();
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

			// A page the NPC says without the server, as v83 does when
			// there is nothing to list (FUN_006d3392). OK and END CHAT
			// both just close it.
			void say(int32_t npc_id, const std::string& text)
			{
				begin({ QuestDialogue::reading({ text }), 0, npc_id });
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
					talk->change_text(active->npc_id, type, style, page->speaker, page->text, {}, UINpcTalk::Menu::LIST);
					talk->set_local_responder(respond);
				}
			}
		}

		void open(int16_t quest_id, int32_t npc_id)
		{
			const QuestInfo* quest = QuestCatalog::get().find(quest_id);

			if (!quest)
				return;

			// A completed quest may be due to repeat.
			if (state(quest_id) != 1)
			{
				if (quest->start_npc != npc_id || !can_start(*quest))
					return;

				if (quest->start_script)
				{
					// The server runs this step as a script and sends its dialogue.
					QuestActionPacket(QuestActionPacket::SCRIPT_START, quest_id, npc_id, player().get_position()).dispatch();
					return;
				}

				// An autoAccept quest starts without its offer (FUN_00717182).
				QuestScript script = quest->auto_accept ? QuestScript() : QuestCatalog::get().script(quest_id, false);
				begin({ QuestDialogue(QuestDialogue::Mode::OFFER, std::move(script)), quest_id, npc_id });
				return;
			}

			// v83 checks Check.img/<id>/1 before saying anything (FUN_00716fe1).
			QuestScript script = QuestCatalog::get().script(quest_id, true);
			QuestStop stop = completion_stop(*quest, npc_id);

			if (stop == QuestStop::NO_PET || stop == QuestStop::WRONG_PET)
			{
				UI::get().emplace<UIOk>(stop == QuestStop::NO_PET ? NO_PET : WRONG_PET, [](bool) {});
				return;
			}

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
			QuestMenu menu = select_quest_menu(npc_id, QuestCatalog::get().quests(),
				[](int16_t id) { return state(id); }, can_start,
				[npc_id](const QuestInfo& quest) { return completion_stop(quest, npc_id) == QuestStop::NONE; });

			// As v83 decides (FUN_0095fe9e, FUN_006d3392): the server runs an
			// NPC without quests, or one with a script and nothing to list;
			// otherwise the NPC says its own line here.
			std::string script = npc_script(npc_id);

			if (menu.quests == 0 || (menu.empty() && !script.empty()))
			{
				TalkToNPCPacket(object_id).dispatch();
				return;
			}

			if (menu.quests == menu.finished)
			{
				say(npc_id, npc_line(npc_id, "d1"));
				return;
			}

			std::string greeting = npc_line(npc_id, "d0");

			if (menu.empty())
			{
				if (!greeting.empty())
					say(npc_id, greeting);

				return;
			}

			// The greeting, then the quests in v83's order, under
			// UtilDlgEx/list3 "QUEST THAT CAN BE COMPLETED", list1 "QUEST
			// AVAILABLE" and list0 "QUEST IN PROGRESS", then the script under
			// list2 "ETC", named by Etc/ScriptInfo.img or else by itself.
			std::string text = greeting;
			std::map<int32_t, Texture> headers;

			for (auto [group, header] : { std::pair<const std::vector<int16_t>*, int>{ &menu.completable, 3 },
					{ &menu.available, 1 }, { &menu.in_progress, 0 } })
			{
				for (size_t i = 0; i < group->size(); i++)
				{
					int16_t id = (*group)[i];

					if (i == 0)
						headers[id] = list_header(header);

					text += quest_menu_line(id, *QuestCatalog::get().find(id), header != 1, header == 3);
				}
			}

			if (!script.empty())
			{
				std::string name = nl::nx::etc["ScriptInfo.img"][script].get_string(script);

				headers[TALK_CHOICE] = list_header(2);
				text += "#d#L" + std::to_string(TALK_CHOICE) + "# " + name + "#l#k\r\n";
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

		bool startable(const QuestInfo& quest)
		{
			return can_start(quest);
		}

		bool completable(const QuestInfo& quest, int32_t npc_id)
		{
			return state(quest.id) == 1 && completion_stop(quest, npc_id) == QuestStop::NONE;
		}

		void alert_menu(const std::vector<quest_alert::Entry>& entries, std::function<void(int16_t)> chosen)
		{
			// The quests under the same headers as an NPC's menu: list3 completable, list1 startable, list0 in
			// progress.
			std::string text = ALERT_GREETING;
			std::map<int32_t, Texture> headers;
			int header = -1;

			for (const quest_alert::Entry& entry : entries)
			{
				const QuestInfo* quest = QuestCatalog::get().find(entry.quest);

				if (!quest)
					continue;

				if (entry.header != header)
				{
					header = entry.header;
					headers[entry.quest] = list_header(header);
				}

				text += quest_menu_line(entry.quest, *quest, header != 1, header == 3);
			}

			active.reset();

			UI::get().emplace<UINpcTalk>();
			UI::get().enable();

			if (auto talk = UI::get().get_element<UINpcTalk>())
			{
				talk->change_text(ALERT_NPC, 4, 0, 0, text, headers, UINpcTalk::Menu::LIST);
				talk->set_local_responder([chosen](int8_t response, int32_t selection)
				{
					if (response == 1)
						chosen(static_cast<int16_t>(selection));
				});
			}
		}
	}
}
