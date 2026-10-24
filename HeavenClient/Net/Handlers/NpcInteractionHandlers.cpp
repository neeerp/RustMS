//////////////////////////////////////////////////////////////////////////////////
//	This file is part of the continued Journey MMORPG client					//
//	Copyright (C) 2015-2019  Daniel Allendorf, Ryan Payton						//
//																				//
//	This program is free software: you can redistribute it and/or modify		//
//	it under the terms of the GNU Affero General Public License as published by	//
//	the Free Software Foundation, either version 3 of the License, or			//
//	(at your option) any later version.											//
//																				//
//	This program is distributed in the hope that it will be useful,				//
//	but WITHOUT ANY WARRANTY; without even the implied warranty of				//
//	MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the				//
//	GNU Affero General Public License for more details.							//
//																				//
//	You should have received a copy of the GNU Affero General Public License	//
//	along with this program.  If not, see <https://www.gnu.org/licenses/>.		//
//////////////////////////////////////////////////////////////////////////////////
#include "NpcInteractionHandlers.h"
#include "ShopList.h"
#include "StoreBank.h"
#include "../../Configuration.h"

#include "../Character/QuestResult.h"
#include "../Data/ItemData.h"
#include "../Gameplay/Stage.h"
#include "../IO/UI.h"
#include "../IO/UITypes/UIAvatarSelect.h"
#include "../IO/UITypes/UIChatBar.h"
#include "../IO/UITypes/UIDuey.h"
#include "../IO/UITypes/UIInviteCard.h"
#include "../IO/UITypes/UINotice.h"
#include "../IO/UITypes/UINpcTalk.h"
#include "../IO/UITypes/UIPetChooser.h"
#include "../IO/UITypes/UIQuiz.h"
#include "../IO/UITypes/UIRpsGame.h"
#include "../IO/UITypes/UIShop.h"
#include "../IO/UITypes/UISlideMenu.h"
#include "../IO/UITypes/UIStorage.h"
#include "../IO/UITypes/UIStoreBank.h"
#include "../IO/UITypes/UITimedQuests.h"
#include "../Gameplay/QuestConversation.h"
#include "../Net/Packets/NpcInteractionPackets.h"

#include <nlnx/nx.hpp>

#include <algorithm>

namespace ms
{
	void NpcDialogueHandler::handle(InPacket& recv) const
	{
		recv.skip(1);

		int32_t npcid = recv.read_int();
		// 0 say, 1 yes/no, 2 text, 3 number, 4 menu, 5 quiz, 6 speed quiz, 7/8 styles, 9/10 pets,
		// 12 accept/decline, 13 box text, 14 slide menu
		int8_t msgtype = recv.read_byte();
		int8_t speaker = recv.read_byte();

		// A say page with 0x4 speaks for another NPC (v83 7467b8).
		if (msgtype == 0 && (speaker & 0x4))
			npcid = recv.read_int();

		// A quiz (v83 a26b09) or speed quiz (a26c66) has no text and is no part of the conversation. A flag of 0
		// shows one, unless one is open already; 1 closes it without an answer.
		if (msgtype == 5 || msgtype == 6)
		{
			int8_t flag = recv.read_byte();

			if (msgtype == 5 && flag == 0)
			{
				std::string title = recv.read_string();
				std::string question = recv.read_string();
				std::string clue = recv.read_string();
				int32_t min_letters = recv.read_int();
				int32_t max_letters = recv.read_int();
				int32_t seconds = recv.read_int();

				auto quiz = UI::get().get_element<UIQuiz>();

				if (!quiz || !quiz->is_active())
					UI::get().emplace<UIQuiz>(title, question, clue, min_letters, max_letters, seconds);
			} else if (msgtype == 6 && flag == 0)
			{
				int32_t subject_type = recv.read_int();
				int32_t subject_id = recv.read_int();
				int32_t points = recv.read_int();
				int32_t number = recv.read_int();
				int32_t seconds = recv.read_int();

				auto quiz = UI::get().get_element<UISpeedQuiz>();

				if (!quiz || !quiz->is_active())
					UI::get().emplace<UISpeedQuiz>(subject_type, subject_id, points, number, seconds);
			} else if (flag == 1)
			{
				if (msgtype == 5)
				{
					if (auto quiz = UI::get().get_element<UIQuiz>())
						quiz->hide();
				} else if (auto quiz = UI::get().get_element<UISpeedQuiz>())
				{
					quiz->hide();
				}
			}

			UI::get().enable();

			return;
		}

		// A slide menu (v83 747a46) has no text: its art's set, then its choices.
		if (msgtype == 14)
		{
			int32_t menutype = recv.read_int();
			slide_menu::Menu menu = slide_menu::parse(recv.read_string());

			if (auto npctalk = UI::get().get_element<UINpcTalk>())
				npctalk->deactivate();

			UI::get().emplace<UISlideMenu>(menutype, std::move(menu));
			UI::get().enable();

			return;
		}

		std::string text = recv.read_string();

		// A choice of styles: the styles, then the same reply whichever.
		if (msgtype == 7 || msgtype == 8)
		{
			std::vector<int32_t> styles(static_cast<uint8_t>(recv.read_byte()));

			for (int32_t &id : styles)
				id = recv.read_int();

			if (styles.empty())
				return;

			if (auto npctalk = UI::get().get_element<UINpcTalk>())
				npctalk->deactivate();

			UI::get().emplace<UIAvatarSelect>(npcid, msgtype, text, std::move(styles));
			UI::get().enable();

			return;
		}

		// A choice of pets (v83 7474a2, 74775c): their Cash Shop serials, each with a byte v83 ignores. Type 10
		// has a byte first which, when 0, needs two pets rather than one. Pets not in the Cash tab are left
		// out, and with too few v83 answers 2 without asking.
		if (msgtype == 9 || msgtype == 10)
		{
			uint8_t count = static_cast<uint8_t>(recv.read_byte());
			size_t least = msgtype == 10 && recv.read_byte() == 0 ? 2 : 1;

			const Inventory &inventory = Stage::get().get_player().get_inventory();
			std::vector<UIPetChooser::Pet> pets;
			std::vector<int64_t> serials;

			for (uint8_t i = 0; i < count; i++)
			{
				int64_t serial = recv.read_long();
				recv.skip(1);

				if (Optional<const Pet> pet = inventory.get_pet(serial))
				{
					bool dead = item_expiry::pet_dead(pet->get_date_dead(), pet->get_remain_life(),
						ItemData::get(pet->get_item_id()).get_pet_life());
					pets.push_back({ pet->get_item_id(), pet->get_name(), pet->get_level(),
									 static_cast<int16_t>(pet->get_closeness()), dead });
					serials.push_back(serial);
				}
			}

			if (pets.size() < least)
			{
				NpcTalkMorePacket(msgtype, 2).dispatch();
				return;
			}

			if (auto npctalk = UI::get().get_element<UINpcTalk>())
				npctalk->deactivate();

			std::string caption = NpcTextMarkup::expand(text, UINpcTalk::markup_lookups());

			UI::get().emplace<UIPetChooser>(caption, std::move(pets), [msgtype, serials](size_t index)
			{
				NpcTalkPetPacket(msgtype, serials[index]).dispatch();
			}, [msgtype]()
			{
				NpcTalkMorePacket(msgtype, 0).dispatch();
			});
			UI::get().enable();

			return;
		}

		int16_t style = 0;
		UINpcTalk::Input input;

		switch (msgtype)
		{
			case 0:
				if (recv.length() > 0)
					style = recv.read_short();

				break;
			case 2:
				// The default, then the fewest and most bytes.
				input.text = recv.read_string();
				input.min = recv.read_short();
				input.max = recv.read_short();
				break;
			case 3:
				// The default, then the range.
				input.text = std::to_string(std::max(recv.read_int(), 0));
				input.min = recv.read_int();
				input.max = recv.read_int();
				break;
			case 13:
				// The default, then the box's columns and rows.
				input.text = recv.read_string();
				input.cols = recv.read_short();
				input.rows = recv.read_short();
				break;
		}

		UI::get().emplace<UINpcTalk>();
		UI::get().enable();

		if (auto npctalk = UI::get().get_element<UINpcTalk>())
			npctalk->change_text(npcid, msgtype, style, speaker, text, {}, UINpcTalk::Menu::INLINE, input);
	}

	namespace
	{
		// The notice for a quest step whose items don't fit: the step the
		// quest is at, taken from Act.img for this character.
		std::string inventory_full(int16_t quest_id)
		{
			Player& player = Stage::get().get_player();
			std::string name = std::to_string(quest_id);
			nl::node act = nl::nx::quest["Act.img"][name][player.get_quests().is_started(quest_id) ? "1" : "0"];
			std::vector<quest_result::ActItem> items;

			for (size_t i = 0; act["item"][std::to_string(i)]; i++)
			{
				nl::node item = act["item"][std::to_string(i)];
				items.push_back({ static_cast<int32_t>(item["id"].get_integer()),
					static_cast<int32_t>(item["count"].get_integer()),
					static_cast<int32_t>(item["job"].get_integer(-1)),
					static_cast<int32_t>(item["gender"].get_integer(2)) });
			}

			const CharStats& stats = player.get_stats();

			return quest_result::inventory_full(items, static_cast<int32_t>(act["money"].get_integer()),
				static_cast<int16_t>(stats.get_job().get_id()), stats.get_female());
		}
	}

	namespace
	{
		// Mode 6: put up the clocks, each with its time left of the quest's timeLimit (FUN_00a24d5d).
		void put_up_clocks(InPacket& recv)
		{
			int16_t count = recv.read_short();

			for (int16_t i = 0; i < count; i++)
			{
				int16_t quest = recv.read_short();
				int32_t ms = recv.read_int();
				auto total = static_cast<int32_t>(
					nl::nx::quest["QuestInfo.img"][std::to_string(quest)]["timeLimit"].get_integer() * 1000);

				timed_quests::add(timed_quests::shown(), quest, total > 0 ? total : ms, ms);
			}

			// One widget shows all of shown(); a second would leak with its alarms ringing.
			if (!UI::get().get_element<UITimedQuests>())
				UI::get().emplace<UITimedQuests>();
		}

		void take_down_clock(int16_t quest)
		{
			if (auto clocks = UI::get().get_element<UITimedQuests>())
				clocks->taken_down(quest);
			else
				timed_quests::remove(timed_quests::shown(), quest);
		}
	}

	void UpdateQuestInfoHandler::handle(InPacket& recv) const
	{
		std::string notice;

		switch (recv.read_byte())
		{
		case 6:
			put_up_clocks(recv);

			return;
		case 7:
		{
			int16_t count = recv.read_short();

			for (int16_t i = 0; i < count; i++)
				take_down_clock(recv.read_short());

			return;
		}
		case 8:
		{
			int16_t quest_id = recv.read_short();
			int32_t npc_id = recv.read_int();
			int16_t next_quest = recv.length() >= 2 ? recv.read_short() : 0;

			QuestConversation::confirmed(quest_id, npc_id, next_quest);

			return;
		}
		case 9:
			notice = "The quest has ended\\ndue to an unknown error.";
			break;
		case 10:
			notice = inventory_full(recv.read_short());
			break;
		case 11:
			notice = "You do not have enough mesos.";
			break;
		case 13:
			notice = "Unable to retrieve it due to the equipment\\n currently being worn by the character.";
			break;
		case 14:
			notice = "You may not possess more than \\none of this item.";
			break;
		case 15:
		{
			int16_t quest = recv.read_short();
			take_down_clock(quest);

			std::string quest_id = std::to_string(quest);
			std::string name = nl::nx::quest["QuestInfo.img"][quest_id]["name"].get_string();

			if (auto chatbar = UI::get().get_element<UIChatBar>())
				chatbar->send_chatline("The [" + name + "] quest expired because the time limit ended",
					UIChatBar::LineType::GREY);

			return;
		}
		case 16:
			timed_quests::reset(timed_quests::shown(), recv.read_short());

			return;
		default:
			// 12 keeps a quest timer, which the server doesn't have.
			return;
		}

		UI::get().emplace<UIOk>(notice, [](bool) {});
	}

	void OpenNpcShopHandler::handle(InPacket& recv) const
	{
		int32_t npcid = recv.read_int();
		auto oshop = UI::get().get_element<UIShop>();

		if (!oshop)
			return;

		UIShop& shop = *oshop;

		shop.reset(npcid);

		for (const shop_list::Row& row : shop_list::read(recv))
			shop.add_item(row);
	}

	void ConfirmShopTransactionHandler::handle(InPacket& recv) const
	{
		// The notice v83 shows for each code (FUN_00756da7). 0, 4 and 8 went through, and a code it doesn't know is an
		// error.
		std::string message = "Due to an error, the trade did not happen.";

		switch (recv.read_byte())
		{
		case 0:
		case 4:
		case 8:
			return;
		case 1:
		case 5:
		case 9:
			message = "You don't have enough in stock.";
			break;
		case 2:
		case 10:
			message = "You do not have enough mesos.";
			break;
		case 3:
			message = "Please check if your inventory is full or not.";
			break;
		case 13:
			message = "You need more items";
			break;
		case 14:
			// v83 shows the level negated.
			message = "You must be under lv." + std::to_string(-recv.read_int()) + " to purchase this item";
			break;
		case 15:
			message = "You must be over lv." + std::to_string(recv.read_int()) + " to purchase this item";
			break;
		case 16:
			message = "Players that are Level 15 and below \\nmay only trade 1 million mesos per day. \\nYou have reached the "
					"limit today,\\nplease try again tomorrow.";
			break;
		case 17:
			if (recv.read_bool())
				message = recv.read_string();

			break;
		}

		UI::get().emplace<UIOk>(message, [](bool) {});
	}

	void TrunkResultHandler::handle(InPacket& recv) const
	{
		enum Mode : int8_t
		{
			TAKEN_OUT = 0x09,
			INVENTORY_FULL = 0x0a,
			NO_MESOS_TO_TAKE = 0x0b,
			ONLY_ONCE = 0x0c,
			STORED = 0x0d,
			SORTED = 0x0f,
			NO_MESOS_TO_STORE = 0x10,
			STORAGE_FULL = 0x11,
			MESO = 0x13,
			OPEN = 0x16,
			MESSAGE = 0x17
		};

		int8_t mode = recv.read_byte();
		auto ostorage = UI::get().get_element<UIStorage>();

		if (!ostorage)
			return;

		UIStorage& storage = *ostorage;

		if (mode == Mode::OPEN)
		{
			int32_t npcid = recv.read_int();
			storage.open(npcid, storage_contents::read(recv));

			return;
		}

		if (!storage.is_active())
			return;

		std::string message = "Due to an error, the trade did not happen.";

		switch (mode)
		{
		case Mode::TAKEN_OUT:
		case Mode::STORED:
		case Mode::SORTED:
		case Mode::MESO:
			storage.change(storage_contents::read(recv));
			return;
		case Mode::INVENTORY_FULL:
			message = "Please check if your inventory is full or not.";
			break;
		case Mode::ONLY_ONCE:
			message = "Item could not be retrieved\\nbecause there was an item that\\ncould only be acquired once.";
			break;
		case Mode::NO_MESOS_TO_TAKE:
		case Mode::NO_MESOS_TO_STORE:
			message = "You do not have enough mesos.";
			break;
		case Mode::STORAGE_FULL:
			message = "The storage is full.";
			break;
		case Mode::MESSAGE:
			if (recv.read_bool())
				message = recv.read_string();

			break;
		}

		storage.answered();
		UI::get().emplace<UIOk>(message, [](bool) {});
	}

	void StoreBankMessageHandler::handle(InPacket& recv) const
	{
		if (auto window = UI::get().get_element<UIStoreBank>())
			window->answered(recv.read_byte());
	}

	void StoreBankHandler::handle(InPacket& recv) const
	{
		int8_t mode = recv.read_byte();

		switch (mode)
		{
		case store_bank::Mode::OPEN:
		{
			int32_t npcid = recv.read_int();
			storage_contents::Contents contents = storage_contents::read(recv);
			UI::get().emplace<UIStoreBank>();

			if (auto window = UI::get().get_element<UIStoreBank>())
				window->open(npcid, contents);

			break;
		}
		case store_bank::Mode::FEE:
		{
			int32_t days = recv.read_int();
			int32_t fee = recv.read_int();

			if (auto window = UI::get().get_element<UIStoreBank>())
				window->quote(days, fee);

			break;
		}
		case store_bank::Mode::ELSEWHERE:
		{
			int32_t npcid = recv.read_int();
			int32_t mapid = recv.read_int();
			auto channel = static_cast<uint8_t>(recv.read_byte());
			Configuration& config = Configuration::get();
			std::string text = store_bank::elsewhere_text(mapid, channel,
				config.get_channel_name(config.get_worldid(), channel));

			UI::get().emplace<UINpcTalk>();
			UI::get().enable();

			if (auto talk = UI::get().get_element<UINpcTalk>())
			{
				talk->change_text(npcid, UINpcTalk::SENDOK, 0, 0, text);
				talk->set_local_responder([](int8_t, int32_t) {});
			}

			break;
		}
		case store_bank::Mode::UNAVAILABLE:
			// Not in v83, which leaves a Get All waiting: RustMS answers
			// one that comes to nothing with this.
			if (auto window = UI::get().get_element<UIStoreBank>())
				window->answered_nothing();

			UI::get().emplace<UIOk>(store_bank::UNAVAILABLE_TEXT, [](bool) {});
			break;
		}
	}

	void RpsGameHandler::handle(InPacket& recv) const
	{
		enum Mode : uint8_t
		{
			NO_MESOS = 6,
			NO_SLOT = 7,
			OPEN = 8,
			START = 9,
			TIME_OVER = 10,
			RESULT = 11,
			NEXT_ROUND = 12,
			CLOSE = 13,
			RESET = 14
		};

		auto mode = static_cast<uint8_t>(recv.read_byte());
		auto window = UI::get().get_element<UIRpsGame>();
		bool open = window && window->is_active();

		// 0x73fff1: everything but the opening needs the window.
		if (mode == Mode::OPEN)
		{
			if (open)
				return;

			int32_t npc_id = recv.read_int();

			// The notice comes first, and the window once it is answered either way (StringPool 0xe5b).
			UI::get().emplace<UINpcTalk>();
			UI::get().enable();

			if (auto talk = UI::get().get_element<UINpcTalk>())
			{
				talk->change_text(npc_id, UINpcTalk::TalkType::SENDOK, 0, 0, UIRpsGame::NOTICE);
				talk->set_local_responder([npc_id](int8_t, int32_t)
				{
					UI::get().emplace<UIRpsGame>(npc_id);
				});
			}

			return;
		}

		if (!open)
			return;

		switch (mode)
		{
			case Mode::NO_MESOS:
			case Mode::NO_SLOT:
			case Mode::RESET:
				window->refused(mode == Mode::NO_MESOS ? UIRpsGame::Refusal::NO_MESOS
					: mode == Mode::NO_SLOT ? UIRpsGame::Refusal::NO_SLOT
					: UIRpsGame::Refusal::SILENT);
				break;
			case Mode::START:
			case Mode::NEXT_ROUND:
				window->start_round();
				break;
			case Mode::TIME_OVER:
				window->time_over();
				break;
			case Mode::RESULT:
			{
				int8_t hand = recv.read_byte();
				int8_t streak = recv.read_byte();
				window->result(hand, streak);
				break;
			}
			case Mode::CLOSE:
				window->close();
				break;
			default:
				break;
		}
	}

	void ParcelHandler::handle(InPacket& recv) const
	{
		enum Mode : uint8_t
		{
			OPEN = 0x08,
			REMOVED = 0x17,
			ADDED = 0x18,
			ARRIVED = 0x19,
			QUICK_SEND = 0x1a,
			WAITING = 0x1b
		};

		auto mode = static_cast<uint8_t>(recv.read_byte());
		auto duey = UI::get().get_element<UIDuey>();
		bool open = duey && duey->is_active();

		switch (mode)
		{
			case Mode::OPEN:
			{
				// v83 can't take a second window (FUN_006f56ea).
				if (open)
					return;

				bool receive_only = recv.read_bool();
				UI::get().emplace<UIDuey>(receive_only ? UIDuey::Kind::RECEIVE : UIDuey::Kind::PARCELS);

				if (auto window = UI::get().get_element<UIDuey>())
					window->open(parcels::read_lists(recv));

				return;
			}
			case Mode::REMOVED:
				if (open)
				{
					int32_t id = recv.read_int();
					duey->removed(id, recv.read_byte() == 3);
				}

				return;
			case Mode::ADDED:
				if (open)
					duey->added(parcels::read(recv));

				return;
			case Mode::ARRIVED:
			case Mode::WAITING:
			{
				// Mode 0x19 names who sent it; 0x1b says some wait.
				std::string sender = mode == Mode::ARRIVED ? recv.read_string() : "";
				bool quick = recv.read_bool();

				if (open)
					UI::get().raise(UIDuey::TYPE);
				else if (auto cards = UI::get().get_element<UIInviteCard>())
					cards->parcel(sender, quick);

				return;
			}
			case Mode::QUICK_SEND:
				// The Quick Delivery Coupon's window, alone.
				if (!open)
					UI::get().emplace<UIDuey>(UIDuey::Kind::QUICK_SEND);

				return;
			default:
				if (open)
					duey->answered(mode);

				return;
		}
	}
}
