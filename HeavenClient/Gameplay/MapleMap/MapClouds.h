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

#include "CloudLayout.h"

#include "../../Graphics/Texture.h"
#include "../../Template/Range.h"

#include <vector>

namespace ms
{
	// The clouds drifting over a map with info/cloud set (v83 FUN_0063f671).
	class MapClouds
	{
	public:
		// Scatter clouds over a field's physical space.
		MapClouds(Range<int16_t> spacex, Range<int16_t> spacey);
		// A map without clouds.
		MapClouds();

		void update();

		void draw(double viewx, double viewy, float alpha) const;

	private:
		std::vector<Texture> textures;
		std::vector<MapCloud> clouds;
		map_clouds::Area area{};
		// The ms since the map loaded.
		int64_t elapsed = 0;
	};
}
