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

#include "../../Gameplay/Stage.h"
#include "../../IO/UI.h"
#include "../../IO/UITypes/UICashShop.h"
#include "../../IO/Window.h"

#include "Timer.h"

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

				items.push_back({ cash_id, item_id, quantity });
			}

			recv.skip_short();	// storage slots
			recv.skip_short();	// character slots

			cashshop->set_cash_items(items);
			break;
		}
		case 0x4D:
		{
			// Gifts waiting for the account join the Cash Inventory.
			int16_t count = recv.read_short();
			std::vector<UICashShop::CashItem> items;

			for (int16_t i = 0; i < count; i++)
			{
				int64_t cash_id = recv.read_long();
				int32_t item_id = recv.read_int();
				recv.skip(13);	// from
				recv.skip(73);	// message

				items.push_back({ cash_id, item_id, 1 });
			}

			cashshop->add_cash_items(items);
			break;
		}
		case 0x4F:
		{
			std::vector<int32_t> sns;

			for (size_t i = 0; i < 10; i++)
				sns.push_back(recv.read_int());

			cashshop->set_wish_list(sns);
			break;
		}
		default:
			break;
		}
	}
}
