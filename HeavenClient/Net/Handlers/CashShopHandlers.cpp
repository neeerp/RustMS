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
#include "CashShopHandlers.h"

#include "Helpers/CashShopParser.h"
#include "Helpers/ItemParser.h"

#include "../../Data/ItemData.h"
#include "../../Gameplay/Stage.h"
#include "../../IO/UI.h"
#include "../../IO/UITypes/UICashShop.h"
#include "../../IO/UITypes/UINotice.h"
#include "../../IO/Window.h"

#include "Timer.h"

#include <algorithm>

namespace ms
{
	void SetCashShopHandler::handle(InPacket& recv) const
	{
		CashShopParser::parseCharacterInfo(recv);

		recv.skip_byte();	// Not MTS
		std::string account = recv.read_string();
		recv.skip_int();

		int16_t specialcashitem_size = recv.read_short();

		for (size_t i = 0; i < specialcashitem_size; i++)
		{
			recv.skip_int();	// sn
			recv.skip_int();	// mod
			recv.skip_byte();	// info
		}

		recv.skip(121);

		// The Best Items of each category, for each gender. The column is the
		// same on every tab, so keep the first category's for the player.
		bool female = Stage::get().get_player().get_stats().get_female();
		std::vector<int32_t> best;

		for (size_t cat = 1; cat <= 8; cat++)
		{
			for (size_t gender = 0; gender < 2; gender++)
			{
				for (size_t in = 0; in < 5; in++)
				{
					recv.skip_int(); // category
					recv.skip_int(); // gender
					int32_t sn = recv.read_int();

					if (cat == 1 && gender == female)
						best.push_back(sn);
				}
			}
		}

		recv.skip_int();
		recv.skip_short();
		recv.skip_byte();
		recv.skip_int();

		transition();

		UI::get().change_state(UI::State::CASHSHOP);

		if (auto cashshop = UI::get().get_element<UICashShop>())
			cashshop->set_account(account, best);
	}

	void SetCashShopHandler::transition() const
	{
		float fadestep = 0.025f;

		Window::get().fadeout(
			fadestep,
			[]()
			{
				GraphicsGL::get().clear();

				Stage::get().load(-1, 0);

				UI::get().enable();
				Timer::get().start();
				GraphicsGL::get().unlock();
			}
		);

		GraphicsGL::get().lock();
		Stage::get().clear();
		Timer::get().start();
	}

	void QueryCashResultHandler::handle(InPacket& recv) const
	{
		int32_t credit = recv.read_int();
		int32_t points = recv.read_int();
		int32_t prepaid = recv.read_int();

		if (auto cashshop = UI::get().get_element<UICashShop>())
			cashshop->set_balance(credit, points, prepaid);
	}

	namespace
	{
		// An item in the Cash Inventory, as CASHSHOP_OPERATION sends it.
		UICashShop::CashItem read_cash_item(InPacket& recv)
		{
			int64_t cash_id = recv.read_long();
			recv.skip_int();	// account id
			recv.skip_int();	// character id
			int32_t item_id = recv.read_int();
			recv.skip_int();	// sn
			int16_t quantity = recv.read_short();
			recv.skip(13);		// gift from
			recv.skip_long();	// expiration
			recv.skip_long();

			return { cash_id, item_id, quantity };
		}

		// A player's text in a notice, without the characters that would
		// format it.
		std::string plain(std::string text)
		{
			std::replace(text.begin(), text.end(), '#', ' ');
			std::replace(text.begin(), text.end(), '\\', '/');

			return text;
		}

		std::vector<int32_t> read_wish_list(InPacket& recv)
		{
			std::vector<int32_t> sns;

			for (size_t i = 0; i < 10; i++)
				sns.push_back(recv.read_int());

			return sns;
		}

		std::string failure_message(uint8_t code)
		{
			switch (code)
			{
			case 0xA5:
				return "You don't have enough cash.";
			case 0xA8:
				return "You cannot send a gift to your own account.";
			case 0xA9:
				return "Please confirm whether the character's name is correct.";
			case 0xAA:
				return "Due to gender restrictions, you can't buy this item.";
			case 0xBB:
				return "Please check if your inventory is full or not.";
			case 0xC0:
				return "This item is out of stock, and therefore not available for sale.";
			case 0xC4:
				return "Please check your birthday code.";
			default:
				return "Due to an unknown error, failed.";
			}
		}
	}

	void CashShopOperationHandler::handle(InPacket& recv) const
	{
		auto cashshop = UI::get().get_element<UICashShop>();

		if (!cashshop)
			return;

		int8_t mode = recv.read_byte();

		switch (mode)
		{
		case 0x4B:
		{
			// The Cash Inventory
			int16_t count = recv.read_short();
			std::vector<UICashShop::CashItem> items;

			for (int16_t i = 0; i < count; i++)
				items.push_back(read_cash_item(recv));

			recv.skip_short();	// storage slots
			recv.skip_short();	// character slots

			cashshop->set_cash_items(items);
			break;
		}
		case 0x4D:
		{
			// Gifts received since the last visit join the Cash Inventory.
			int16_t count = recv.read_short();
			std::vector<UICashShop::CashItem> items;
			std::string notice;

			for (int16_t i = 0; i < count; i++)
			{
				int64_t cash_id = recv.read_long();
				int32_t item_id = recv.read_int();
				std::string from = recv.read_padded_string(13);
				std::string message = recv.read_padded_string(73);

				items.push_back({ cash_id, item_id, 1 });

				if (!notice.empty())
					notice += "\\n\\n";

				notice += from + " has sent you " + ItemData::get(item_id).get_name() + ".\\n\"" + plain(message) + "\"";
			}

			cashshop->add_cash_items(items);

			if (!notice.empty())
				UI::get().emplace<UIOk>(notice, [](bool) {});

			break;
		}
		case 0x4F:
		case 0x55:
			// The wish list, on entering and after a change.
			cashshop->set_wish_list(read_wish_list(recv));
			break;
		case 0x57:
			// A purchase is in the Cash Inventory; the balance follows.
			cashshop->add_cash_items({ read_cash_item(recv) });
			UI::get().emplace<UIOk>("You have successfully made the purchase.", [](bool) {});
			break;
		case 0x5E:
		{
			// A gift is on its way; the balance follows.
			std::string to = recv.read_string();
			int32_t item_id = recv.read_int();
			recv.skip_short();	// count
			recv.skip_int();	// price

			UI::get().emplace<UIOk>("You have sent " + ItemData::get(item_id).get_name() + " to " + to + ".", [](bool) {});
			break;
		}
		case 0x68:
		{
			// A Cash Inventory item is now in its bag at slot. Peek at the
			// item's id and serial, which come after its type byte.
			int16_t slot = recv.read_short();
			InPacket peek = recv;
			peek.skip_byte();
			int32_t item_id = peek.read_int();
			int64_t cash_id = peek.read_bool() ? peek.read_long() : 0;

			Inventory& inventory = Stage::get().get_player().get_inventory();
			ItemParser::parse_item(recv, InventoryType::by_item_id(item_id), slot, inventory);
			cashshop->remove_cash_item(cash_id);
			break;
		}
		case 0x6A:
		{
			// A character's item is back in the Cash Inventory.
			UICashShop::CashItem item = read_cash_item(recv);
			InventoryType::Id type = InventoryType::by_item_id(item.item_id);
			Inventory& inventory = Stage::get().get_player().get_inventory();

			if (int16_t slot = inventory.find_cash_item(type, item.cash_id))
				inventory.modify(type, slot, Inventory::Modification::REMOVE, 0, Inventory::Movement::MOVE_NONE);

			cashshop->add_cash_items({ item });
			break;
		}
		case 0x5C:
			UI::get().emplace<UIOk>(failure_message(recv.read_byte()), [](bool) {});
			break;
		default:
			break;
		}
	}
}
