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
#pragma once

#include "../../Graphics/Animation.h"

#include <map>
#include <string>

namespace ms
{
	// The kites flown over the current map (SPAWN_KITE, FUN_0065acdf):
	// an item's `iconReward` above where its flier stood, bobbing, until
	// the server takes it down or the player leaves the map.
	class MapKites
	{
	public:
		struct Kite
		{
			int32_t itemid;
			// The item's own words after the flier's name (String.nx `msg`).
			std::string item_msg;
			std::string message;
			std::string name;
			int16_t x;
			int16_t y;
		};

		void spawn(int32_t oid, int32_t itemid, std::string message, std::string name, int16_t x, int16_t y);

		// Take a kite down, fading it out over a second (REMOVE_KITE type 0)
		// or at once.
		void remove(int32_t oid, bool fade);

		void update();

		void draw(double viewx, double viewy, float alpha) const;

		void clear();

		// The kite under the cursor at `position` (in map coordinates), if
		// any.
		const Kite *at(Point<int16_t> position) const;

	private:
		struct Flying
		{
			Kite kite;
			Animation art;
			double phase;
			// The ms left of its fade, once taken down.
			int16_t fading = -1;
		};

		std::map<int32_t, Flying> kites;
	};
}
