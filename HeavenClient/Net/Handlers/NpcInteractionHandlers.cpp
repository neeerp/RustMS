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

#include "../IO/UI.h"
#include "../IO/UITypes/UINotice.h"
#include "../IO/UITypes/UINpcTalk.h"
#include "../IO/UITypes/UIShop.h"
#include "../IO/UITypes/UIStorage.h"
#include "../Gameplay/QuestConversation.h"

namespace ms
{
	void NpcDialogueHandler::handle(InPacket& recv) const
	{
		recv.skip(1);

		int32_t npcid = recv.read_int();
		int8_t msgtype = recv.read_byte(); // 0 - textonly, 1 - yes/no, 4 - selection, 12 - accept/decline
		int8_t speaker = recv.read_byte();
		std::string text = recv.read_string();

		int16_t style = 0;

		if (msgtype == 0 && recv.length() > 0)
			style = recv.read_short();

		UI::get().emplace<UINpcTalk>();
		UI::get().enable();

		if (auto npctalk = UI::get().get_element<UINpcTalk>())
			npctalk->change_text(npcid, msgtype, style, speaker, text);
	}

	void UpdateQuestInfoHandler::handle(InPacket& recv) const
	{
		int8_t mode = recv.read_byte();

		if (mode != 8)
			return;

		int16_t quest_id = recv.read_short();
		int32_t npc_id = recv.read_int();
		int16_t next_quest = recv.length() >= 2 ? recv.read_short() : 0;

		if (next_quest != 0)
			QuestConversation::continue_with(next_quest, npc_id);
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
		const char* message = nullptr;

		switch (recv.read_byte())
		{
		case 2:
			message = "You do not have enough mesos.";
			break;
		case 3:
			message = "Please check if your inventory is full or not.";
			break;
		default:
			return;
		}

		UI::get().emplace<UIOk>(message, [](bool) {});
	}

	void TrunkResultHandler::handle(InPacket& recv) const
	{
		enum Mode : int8_t
		{
			TAKEN_OUT = 0x09,
			INVENTORY_FULL = 0x0a,
			ONLY_ONCE = 0x0b,
			NO_MESOS_TO_TAKE = 0x0c,
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
}
