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

#include <cmath>
#include <cstdint>

#include "../Constants.h"

#include "Physics/PhysicsObject.h"

namespace ms
{
	struct Movement
	{
		enum Type
		{
			NONE,
			ABSOLUTE,
			RELATIVE,
			CHAIR,
			JUMPDOWN
		};

		Movement(Type t, uint8_t c, int16_t x, int16_t y, int16_t vx, int16_t vy, uint16_t f, uint8_t s, int16_t d)
				: type(t), command(c), xpos(x), ypos(y), vx(vx), vy(vy), fh(f), newstate(s), duration(d)
		{}

		Movement(int16_t x, int16_t y, int16_t vx, int16_t vy, uint8_t s, int16_t d) : Movement(Type::ABSOLUTE, 0, x, y,
																								vx, vy, 0, s, d)
		{}

		// Where the object is and how fast it goes, as element `c`.
		Movement(const PhysicsObject &phobj, uint8_t s, uint8_t c = 0, int16_t d = 1)
				: Movement(Type::ABSOLUTE, c, phobj.get_x(), phobj.get_y(), velocity(phobj.hspeed),
						   velocity(phobj.vspeed), phobj.fhid, s, d)
		{}

		Movement() : Movement(Type::NONE, 0, 0, 0, 0, 0)
		{}

		bool hasmoved(const Movement &newmove) const
		{
			return newmove.newstate != newstate || newmove.xpos != xpos || newmove.ypos != ypos ||
				   newmove.vx != vx || newmove.vy != vy;
		}

		// v83 paths carry speeds in pixels per second; HC's are per step.
		static int16_t velocity(double speed)
		{
			return static_cast<int16_t>(std::lround(speed * 1000 / Constants::TIMESTEP));
		}

		Type type;
		uint8_t command;
		int16_t xpos;
		int16_t ypos;
		// Velocity, in pixels per second.
		int16_t vx;
		int16_t vy;
		uint16_t fh;
		uint8_t newstate;
		int16_t duration;
	};
}