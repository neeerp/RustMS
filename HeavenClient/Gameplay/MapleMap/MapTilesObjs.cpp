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
#include "MapTilesObjs.h"

#include "../../Util/Misc.h"

namespace ms
{
	TilesObjs::TilesObjs(nl::node src)
	{
		auto tileset = src["info"]["tS"] + ".img";

		for (auto tilenode : src["tile"])
		{
			Tile tile{tilenode, tileset};
			int8_t z = tile.getz();
			tiles.emplace(z, std::move(tile));
		}

		for (auto objnode : src["obj"])
		{
			Obj obj{objnode, string_conversion::or_default<int32_t>(objnode.name(), 0)};
			int8_t z = obj.getz();
			objs.emplace(z, std::move(obj));
		}
	}

	TilesObjs::TilesObjs()
	{}

	void TilesObjs::update()
	{
		for (auto &iter : objs)
			iter.second.update();
	}

	const Obj *TilesObjs::find_obstacle(const obstacle::Box &body) const
	{
		const Obj *found = nullptr;

		for (auto &iter : objs)
		{
			const Obj &obj = iter.second;
			obstacle::Box box{};

			if (obj.is_obstacle() && (!found || obj.get_order() > found->get_order()) && obj.obstacle_box(box) &&
				box.meets(body))
				found = &obj;
		}

		return found;
	}

	void TilesObjs::draw(Point<int16_t> viewpos, float alpha) const
	{
		for (auto &iter : objs)
			iter.second.draw(viewpos, alpha);

		for (auto &iter : tiles)
			iter.second.draw(viewpos);
	}

	MapTilesObjs::MapTilesObjs(nl::node src)
	{
		for (auto iter : layers)
			iter.second = src[iter.first];
	}

	MapTilesObjs::MapTilesObjs()
	{}

	void MapTilesObjs::draw(Layer::Id layer, Point<int16_t> viewpos, float alpha) const
	{
		layers[layer].draw(viewpos, alpha);
	}

	const Obj *MapTilesObjs::find_obstacle(const obstacle::Box &body) const
	{
		for (size_t i = Layer::Id::LENGTH; i-- > 0;)
			if (const Obj *obj = layers[static_cast<Layer::Id>(i)].find_obstacle(body))
				return obj;

		return nullptr;
	}

	void MapTilesObjs::update()
	{
		for (auto iter : layers)
			iter.second.update();
	}
}