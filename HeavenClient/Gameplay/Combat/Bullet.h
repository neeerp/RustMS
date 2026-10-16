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

#include "../Graphics/Animation.h"
#include "../Template/Point.h"

namespace ms
{
	// Represents a projectile on a map. It flies straight from where it is
	// thrown or fired to its target in a set time, as in v83.
	class Bullet
	{
	public:
		// A bullet may be turned by `angle` radians, clockwise.
		Bullet(Animation animation, Point<int16_t> origin, Point<int16_t> target, uint16_t duration, float angle = 0.0f);

		void draw(double viewx, double viewy, float alpha) const;

		// Return whether the bullet has arrived.
		bool update();

	private:
		Point<double> get_position(float alpha) const;

		Animation animation;
		Point<int16_t> origin;
		Point<int16_t> target;
		uint16_t duration;
		uint16_t elapsed;
		bool flip;
		float angle;
	};
}
