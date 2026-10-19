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

#include "ObstacleRules.h"

#include "../Graphics/Animation.h"
#include "../Template/Point.h"

namespace ms
{
	// Represents an obj (map decoration) on a map.
	class Obj
	{
	public:
		// `order` is its place among its layer's objs.
		Obj(nl::node source, int32_t order = 0);

		// Update animation.
		void update();

		// Draw the obj at the specified position.
		void draw(Point<int16_t> viewpos, float inter) const;

		// Return depth of the obj.
		uint8_t getz() const;

		int32_t get_order() const;

		// Whether the obj is a trap, which hurts what touches it.
		bool is_obstacle() const;

		const obstacle::Trap &get_trap() const;

		// The box of the trap's current frame on the map (v83 FUN_00642264), or false when the frame has none.
		bool obstacle_box(obstacle::Box &out) const;

		// The push of the trap, where it has a direction.
		obstacle::Vec get_push() const;

	private:
		Animation animation;
		Point<int16_t> pos;
		uint8_t z;
		bool flip;
		int32_t order;
		bool obstacle;
		obstacle::Trap trap;
	};
}