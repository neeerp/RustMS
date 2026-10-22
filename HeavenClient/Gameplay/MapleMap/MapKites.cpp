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
#include "MapKites.h"

#include "KiteRules.h"

#include "../../Constants.h"

#include "../../Util/Misc.h"

#include <nlnx/nx.hpp>

#include "../../Util/Randomizer.h"

#include <cmath>

namespace ms
{
	void MapKites::spawn(int32_t oid, int32_t itemid, std::string message, std::string name, int16_t x, int16_t y)
	{
		nl::node art = nl::nx::item["Cash"]["0508.img"][string_format::extend_id(itemid, 8)]["iconReward"];
		std::string item_msg = nl::nx::string["Cash.img"][std::to_string(itemid)]["msg"];
		double phase = Randomizer().next_int(360);

		kites[oid] = { { itemid, item_msg, std::move(message), std::move(name), x, y }, Animation(art), phase };
	}

	void MapKites::remove(int32_t oid, bool fade)
	{
		auto iter = kites.find(oid);

		if (iter == kites.end())
			return;

		if (!fade)
			kites.erase(iter);
		else if (iter->second.fading < 0)
			iter->second.fading = kite::FADE;
	}

	void MapKites::update()
	{
		for (auto iter = kites.begin(); iter != kites.end();)
		{
			Flying &flying = iter->second;

			flying.art.update();
			flying.phase += kite::BOB_STEP * Constants::TIMESTEP / kite::BOB_MS;

			if (flying.fading >= 0)
			{
				flying.fading -= Constants::TIMESTEP;

				if (flying.fading <= 0)
				{
					iter = kites.erase(iter);
					continue;
				}
			}

			iter++;
		}
	}

	void MapKites::draw(double viewx, double viewy, float alpha) const
	{
		for (const auto &iter : kites)
		{
			const Flying &flying = iter.second;
			const Kite &kite = flying.kite;
			float opacity = flying.fading < 0 ? 1.0f : kite::fade_opacity(flying.fading);
			int16_t bob = static_cast<int16_t>(std::lround(kite::bob(flying.phase)));
			Point<int16_t> position(static_cast<int16_t>(std::round(kite.x + kite::SHIFT + viewx)),
									static_cast<int16_t>(std::round(kite.y - kite::RISE + bob + viewy)));

			flying.art.draw(DrawArgument(position, opacity), alpha);
		}
	}

	void MapKites::clear()
	{
		kites.clear();
	}

	const MapKites::Kite *MapKites::at(Point<int16_t> position) const
	{
		// The newest kite is on top.
		for (auto iter = kites.rbegin(); iter != kites.rend(); ++iter)
		{
			const Kite &kite = iter->second.kite;

			if (kite::hit(kite.x, kite.y, position.x(), position.y()))
				return &kite;
		}

		return nullptr;
	}
}
