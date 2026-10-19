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

#include "../../Gameplay/FieldEffectRules.h"
#include "../../Gameplay/Stage.h"
#include "../../IO/UI.h"

#include "../../IO/UITypes/UILoginNotice.h"
#include "../../IO/UITypes/UIMobGage.h"

#include <nlnx/nx.hpp>

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

		// A summon's puff and sound, louder near the player (FUN_005330f7).
		if (rand == 0)
		{
			uint8_t type = recv.read_byte();
			int32_t x = recv.read_int();
			int32_t y = recv.read_int();

			Stage::get().add_summon_effect(type, Point<int16_t>(x, y));

			return;
		}

		// The view shakes after a delay in ms, a heavy shake fading faster (0x5333eb).
		if (rand == 1)
		{
			bool heavy = recv.read_bool();
			int32_t delay = recv.read_int();

			Stage::get().tremble(30, heavy, delay, 0, false);

			return;
		}

		// A named map obj, such as a party quest's gate, shows its current state (FUN_00642aca with -1).
		if (rand == 2)
		{
			Stage::get().set_object_state(recv.read_string(), -1);

			return;
		}

		// A one-time effect on the screen; packets stack.
		if (rand == 3)
		{
			std::string path = recv.read_string();

			Stage::get().add_effect(path);

			return;
		}

		// A sound from Sound.nx Field.img, played once.
		if (rand == 4)
		{
			nl::node sound = nl::nx::sound["Field.img"].resolve(recv.read_string());

			if (sound)
				Sound(sound).play();

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

		// The BGM until the next map, after the last one fades out.
		if (rand == 6)
		{
			std::string path = field_effect::bgm_path(recv.read_string());

			if (!path.empty())
				Music(path).play(field_effect::BGM_FADE);

			return;
		}

		// The reward roulette's three reels, above the windows. They stop at the first that is missing
		// (FUN_0043d083).
		if (rand == 7)
		{
			int32_t job = recv.read_int();
			int32_t part = recv.read_int();
			int32_t level = recv.read_int();
			Stage &stage = Stage::get();

			if (stage.add_effect("miro/RR1/" + std::to_string(job)) && stage.add_effect("miro/RR2/" + std::to_string(part)))
				stage.add_effect("miro/RR3/" + std::to_string(level));

			return;
		}

		std::cout << "[FieldEffectHandler]: Unknown value: [" << rand << "]" << std::endl;
	}
}