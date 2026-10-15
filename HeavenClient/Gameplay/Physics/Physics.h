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

#include "FootholdTree.h"

namespace ms
{
	// Class that uses physics engines and the collection of platforms to determine object movement
	class Physics
	{
	public:
		// src is the map's foothold node and info its info node.
		Physics(nl::node src, nl::node info = {});

		Physics();

		// Move the specified object over the specified game-time.
		void move_object(PhysicsObject &tomove) const;

		// Determine the point on the ground below the specified position.
		Point<int16_t> get_y_below(Point<int16_t> position) const;

		// Return a reference to the collection of platforms.
		const FootholdTree &get_fht() const;

		// Top walking speed and takeoff speed, in pixels per tick, for the
		// given shoe multipliers.
		static double walk_speed(double walkspeed);

		static double jump_speed(double walkjump);

		// Start a jump from the ground. Holding a direction adds 80% of the
		// top walking speed, as CVecCtrl::Jump does.
		static void jump(PhysicsObject &phobj, double jumpspeed, int8_t dir);

		// Hop off the current platform and fall through it.
		static void jump_down(PhysicsObject &phobj);

	private:
		void move_shoe(PhysicsObject &) const;

		void move_normal(PhysicsObject &) const;

		void move_flying(PhysicsObject &) const;

		void move_swimming(PhysicsObject &) const;

		FootholdTree fht;
	};
}