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

#include <cstdint>

#include "Layer.h"
#include "Tile.h"
#include "Obj.h"

#include "../Template/EnumMap.h"

#include <vector>
#include <map>
#include <functional>
#include <optional>
#include <unordered_map>

namespace ms
{
	// A tile and obj layer.
	class TilesObjs
	{
	public:
		TilesObjs(nl::node src, Point<int16_t> space);

		TilesObjs();

		void draw(Point<int16_t> viewpos, float alpha) const;

		void update();

		// The last-loaded trap in the layer that `body` touches.
		const Obj *find_obstacle(const obstacle::Box &body) const;

		// The obj with this name, or null.
		Obj *find(const std::string &name);

		// Adds the box of each named obj, by its name.
		void add_boxes(std::unordered_map<std::string, Rectangle<int16_t>> &boxes) const;

		// Puts every named trap back in its first state.
		void reset_obstacles(Point<int16_t> listener);

		void show_tags(const std::function<std::optional<bool>(const std::string &)> &shown);

	private:
		std::multimap<uint8_t, Tile> tiles;
		std::multimap<uint8_t, Obj> objs;
	};

	// The collection of tile and obj layers on a map.
	class MapTilesObjs
	{
	public:
		// `space` is the width and height of the map's physical space.
		MapTilesObjs(nl::node src, Point<int16_t> space);

		MapTilesObjs();

		void draw(Layer::Id layer, Point<int16_t> viewpos, float alpha) const;

		void update();

		// The trap that `body` touches. v83 tests them in reverse load order and takes the first (FUN_00642710).
		const Obj *find_obstacle(const obstacle::Box &body) const;

		// Show the named obj in `state`, or in its current one again for -1 (FUN_00642aca). False if there's no
		// such obj or state. Names are unique in each map.
		bool set_state(const std::string &name, int32_t state, Point<int16_t> listener);

		// The box of each named obj, by its name.
		std::unordered_map<std::string, Rectangle<int16_t>> named_boxes() const;

		// Puts every named trap back in its first state (FIELD_OBSTACLE_ALL_RESET 0x8D).
		void reset_obstacles(Point<int16_t> listener);

		// Show or hide each obj with a tag that `shown` gives a value for, leaving the rest as they are.
		void show_tags(const std::function<std::optional<bool>(const std::string &)> &shown);

	private:
		EnumMap<Layer::Id, TilesObjs> layers;
	};
}