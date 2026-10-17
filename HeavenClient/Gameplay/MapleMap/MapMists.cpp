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
#include "MapMists.h"

#include "../../Constants.h"

#include "../../Util/Misc.h"
#include "../../Util/Randomizer.h"

#include <nlnx/nx.hpp>

#include <algorithm>
#include <cmath>

namespace ms
{
	namespace
	{
		constexpr int16_t FADE = 1000;

		nl::node tile_node(int32_t type, int32_t skillid, uint8_t level)
		{
			if (type == 3)
				return nl::nx::skill["MobSkill.img"][string_format::extend_id(skillid, 3)]["level"][std::to_string(level)]["tile"];

			return nl::nx::skill[string_format::extend_id(skillid / 10000, 3) + ".img"]["skill"]
			[string_format::extend_id(skillid, 7)]["tile"];
		}
	}

	void MapMists::spawn(int32_t oid, int32_t type, int32_t skillid, uint8_t level, int32_t left, int32_t top,
						 int32_t right, int32_t bottom)
	{
		nl::node src = tile_node(type, skillid, level);

		std::vector<Animation> tiles;
		std::vector<int16_t> heights;

		for (int32_t i = 0; i < static_cast<int32_t>(src.size()); i++)
		{
			nl::node tile = src[std::to_string(i)];

			if (!tile)
				break;

			tiles.emplace_back(tile);
			heights.push_back(tile["0"].get_bitmap().height());
		}

		Randomizer random;
		std::vector<MistCloud> layout = layout_mist(left, top, right, bottom, heights,
													[&](int32_t n) { return random.next_int(n); });

		// The clouds draw in order of their z; layers of the same z in the
		// order they were made.
		std::stable_sort(layout.begin(), layout.end(),
						 [](const MistCloud &a, const MistCloud &b) { return a.z < b.z; });

		Mist mist;

		for (const MistCloud &cloud : layout)
			mist.clouds.push_back({ tiles[cloud.tile], Point<int16_t>(cloud.x, cloud.y), cloud.flip });

		mists[oid] = std::move(mist);
	}

	void MapMists::remove(int32_t oid)
	{
		auto iter = mists.find(oid);

		if (iter != mists.end() && iter->second.fading < 0)
			iter->second.fading = FADE;
	}

	void MapMists::update()
	{
		for (auto iter = mists.begin(); iter != mists.end();)
		{
			Mist &mist = iter->second;

			for (Cloud &cloud : mist.clouds)
				cloud.animation.update();

			if (mist.fading >= 0)
			{
				mist.fading -= Constants::TIMESTEP;

				if (mist.fading <= 0)
				{
					iter = mists.erase(iter);
					continue;
				}
			}

			++iter;
		}
	}

	void MapMists::draw(double viewx, double viewy, float alpha) const
	{
		Point<int16_t> shift(static_cast<int16_t>(std::round(viewx)), static_cast<int16_t>(std::round(viewy)));

		for (const auto &iter : mists)
		{
			const Mist &mist = iter.second;
			float opacity = mist.fading < 0 ? 1.0f : static_cast<float>(mist.fading) / FADE;

			for (const Cloud &cloud : mist.clouds)
				cloud.animation.draw(DrawArgument(cloud.position + shift, cloud.flip, opacity), alpha);
		}
	}

	void MapMists::clear()
	{
		mists.clear();
	}
}
