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
#include "TestingHandlers.h"

#include "../../Gameplay/Stage.h"
#include "../../IO/UI.h"

#include "../../IO/UITypes/UILoginNotice.h"
#include "../../IO/UITypes/UIMobGage.h"

namespace ms
{
	void CheckSpwResultHandler::handle(InPacket& recv) const
	{
		int reason = recv.read_byte();

		if (reason == 0)
			UI::get().emplace<UILoginNotice>(UILoginNotice::Message::INCORRECT_PIC);
		else
			std::cout << "[CheckSpwResultHandler]: Unknown reason: [" << reason << "]" << std::endl;

		UI::get().enable();
	}

	void FieldEffectHandler::handle(InPacket& recv) const
	{
		int rand = recv.read_byte();

		// Effect
		if (rand == 3)
		{
			std::string path = recv.read_string();

			Stage::get().add_effect(path);

			return;
		}

		// A boss's HP bar, by its template id; an hp under 1 takes it down (0x5332d0).
		if (rand == 5)
		{
			int32_t mobid = recv.read_int();
			int32_t hp = recv.read_int();
			int32_t maxhp = recv.read_int();
			uint8_t color = recv.read_byte();
			uint8_t bgcolor = recv.read_byte();

			UIMobGage::show(mobid, hp, maxhp, color, bgcolor);

			return;
		}

		std::cout << "[FieldEffectHandler]: Unknown value: [" << rand << "]" << std::endl;
	}
}