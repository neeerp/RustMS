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

#include "MistLayout.h"

#include "../../Graphics/Animation.h"

#include <map>

namespace ms
{
	// The mists (affected areas) on a map: clouds of a skill's `tile`
	// animations over an area, such as Poison Mist's.
	class MapMists
	{
	public:
		// Cover an area with a skill's clouds (AFFECTED_AREA_CREATED,
		// FUN_00431a63). A mob's mist (type 3) uses its MobSkill.img level.
		void spawn(int32_t oid, int32_t type, int32_t skillid, uint8_t level, int32_t left, int32_t top,
				   int32_t right, int32_t bottom);

		// Fade a mist's clouds out over a second (FUN_0043234d).
		void remove(int32_t oid);

		void update();

		void draw(double viewx, double viewy, float alpha) const;

		void clear();

	private:
		struct Cloud
		{
			Animation animation;
			Point<int16_t> position;
			bool flip;
		};

		struct Mist
		{
			std::vector<Cloud> clouds;
			// The ms left of its fade, once removed.
			int16_t fading = -1;
		};

		std::map<int32_t, Mist> mists;
	};
}
