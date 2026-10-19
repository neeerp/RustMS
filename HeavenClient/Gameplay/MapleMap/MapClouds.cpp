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
#include "MapClouds.h"

#include "../../Constants.h"

#include "../../Util/Randomizer.h"

#include <nlnx/nx.hpp>

#include <algorithm>
#include <cmath>

namespace ms
{
	MapClouds::MapClouds(Range<int16_t> spacex, Range<int16_t> spacey)
	{
		nl::node src = nl::nx::map["MapHelper.img"]["weather"]["cloud"];

		for (size_t i = 0; i < src.size(); i++)
			textures.emplace_back(src[std::to_string(i)]);

		area = map_clouds::area(spacex.first(), spacey.first(), spacex.second(), spacey.second());

		Randomizer randomizer;
		auto next = [&]() { return randomizer.next_int<int32_t>(32768); };
		clouds = map_clouds::scatter(area, static_cast<int32_t>(textures.size()), next);
	}

	MapClouds::MapClouds()
	{}

	void MapClouds::update()
	{
		if (!clouds.empty())
			elapsed += Constants::TIMESTEP;
	}

	void MapClouds::draw(double viewx, double viewy, float alpha) const
	{
		int64_t at = std::max<int64_t>(0, elapsed - Constants::TIMESTEP + std::lround(alpha * Constants::TIMESTEP));
		int32_t shiftx = static_cast<int32_t>(std::round(viewx));
		int32_t shifty = static_cast<int32_t>(std::round(viewy));

		// v83 draws a cloud's canvas at its point, faded to its opacity from
		// the start.
		for (const MapCloud &cloud : clouds)
		{
			auto [x, y] = map_clouds::position(cloud, at, area);
			Point<int16_t> position(static_cast<int16_t>(x + shiftx), static_cast<int16_t>(y + shifty));
			textures[cloud.tile].draw(DrawArgument(position, cloud.alpha / 255.0f));
		}
	}
}
