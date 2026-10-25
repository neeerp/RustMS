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
	TilesObjs::TilesObjs(nl::node src, Point<int16_t> space)
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
			Obj obj{objnode, string_conversion::or_default<int32_t>(objnode.name(), 0), space};
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

	Obj *TilesObjs::find(const std::string &name)
	{
		for (auto &iter : objs)
			if (iter.second.get_name() == name)
				return &iter.second;

		return nullptr;
	}

	void TilesObjs::add_boxes(std::unordered_map<std::string, Rectangle<int16_t>> &boxes) const
	{
		for (auto &iter : objs)
			if (!iter.second.get_name().empty())
				boxes.emplace(iter.second.get_name(), iter.second.get_box());
	}

	void TilesObjs::reset_obstacles(Point<int16_t> listener)
	{
		for (auto &iter : objs)
		{
			Obj &obj = iter.second;

			if (!obj.get_name().empty() && obj.has_obstacle())
				obj.set_state(0, listener);
		}
	}

	void TilesObjs::show_tags(const std::function<std::optional<bool>(const std::string &)> &shown)
	{
		for (auto &iter : objs)
			for (auto &tag : iter.second.get_tags())
				if (std::optional<bool> value = shown(tag))
					iter.second.set_shown(*value);
	}

	void TilesObjs::draw(Point<int16_t> viewpos, float alpha) const
	{
		for (auto &iter : objs)
			iter.second.draw(viewpos, alpha);

		for (auto &iter : tiles)
			iter.second.draw(viewpos);
	}

	MapTilesObjs::MapTilesObjs(nl::node src, Point<int16_t> space)
	{
		for (auto iter : layers)
			iter.second = TilesObjs(src[iter.first], space);
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

	bool MapTilesObjs::set_state(const std::string &name, int32_t state, Point<int16_t> listener)
	{
		if (name.empty())
			return false;

		for (auto iter : layers)
			if (Obj *obj = iter.second.find(name))
				return obj->set_state(state, listener);

		return false;
	}

	std::unordered_map<std::string, Rectangle<int16_t>> MapTilesObjs::named_boxes() const
	{
		std::unordered_map<std::string, Rectangle<int16_t>> boxes;

		for (auto iter : layers)
			iter.second.add_boxes(boxes);

		return boxes;
	}

	void MapTilesObjs::reset_obstacles(Point<int16_t> listener)
	{
		for (auto iter : layers)
			iter.second.reset_obstacles(listener);
	}

	void MapTilesObjs::show_tags(const std::function<std::optional<bool>(const std::string &)> &shown)
	{
		for (auto iter : layers)
			iter.second.show_tags(shown);
	}

	void MapTilesObjs::update()
	{
		for (auto iter : layers)
			iter.second.update();
	}
}