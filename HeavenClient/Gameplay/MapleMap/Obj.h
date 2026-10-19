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
#include <string>
#include <vector>

#include "ObstacleRules.h"

#include "../../Audio/Audio.h"
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

		// The name the server switches its state by. Most objs have none.
		const std::string &get_name() const;

		// Show a named obj in `state`, or in its current one again for -1, with its sound as heard from `listener`
		// (v83 FUN_00642aca). False if it has no such state.
		bool set_state(int32_t state, Point<int16_t> listener);

		// Whether any of its states is a trap.
		bool has_obstacle() const;

		// Whether the obj is a trap, which hurts what touches it.
		bool is_obstacle() const;

		const obstacle::Trap &get_trap() const;

		// The box of the trap's current frame on the map (v83 FUN_00642264), or false when the frame has none.
		bool obstacle_box(obstacle::Box &out) const;

		// The push of the trap, where it has a direction.
		obstacle::Vec get_push() const;

	private:
		// Its own node, or one of a named obj's s1, s2...
		struct State
		{
			State(nl::node data);

			Animation animation;
			int32_t repeat;
			Sound sound;
			bool obstacle;
			obstacle::Trap trap;
		};

		// Start its animation as its `repeat` says (FUN_006431e3).
		static void play(State &state);

		std::vector<State> states;
		size_t current;
		bool visible;
		std::string name;
		Point<int16_t> pos;
		uint8_t z;
		bool flip;
		int32_t order;
	};
}