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
#include "Drop.h"

#include <algorithm>
#include <cmath>

namespace ms
{
	namespace
	{
		const double DROPSPEED = -2.9;
		const double DROPGRAVITY = 0.042;

		// A picked up drop rises about 50 px, slowing down, and disappears
		// after about 0.37 s, fading only over the last few ticks.
		const double PICKUPSPEED = -2.3;
		const double PICKUPGRAVITY = 0.05;
		const uint16_t PICKUPTIME = 46;
		const uint16_t PICKUPFADETIME = 6;

		// Ticks a falling drop needs to come down by dy pixels.
		double air_ticks(double dy)
		{
			double ticks = (-DROPSPEED + std::sqrt(DROPSPEED * DROPSPEED + 2.0 * DROPGRAVITY * std::max(dy, 0.0))) / DROPGRAVITY;

			return std::max(ticks, 1.0);
		}
	}

	Drop::Drop(int32_t id, int32_t own, Point<int16_t> start, Point<int16_t> dst, int8_t type, int8_t mode, bool pldrp)
			: MapObject(id)
	{
		owner = own;
		set_position(start.x(), start.y() - 4);
		dest = dst;
		pickuptype = type;
		playerdrop = pldrp;

		angle.set(0.0f);
		opacity.set(1.0f);
		moved = 0.0f;
		pickupticks = 0;
		looter = nullptr;

		switch (mode)
		{
			case 0:
			case 1:
				state = Drop::State::DROPPED;
				basey = static_cast<double>(dest.y() - 4);
				// Drops fall under their own, lighter gravity: about 100 px up
				// and about 1.1 s in the air, as in the original.
				phobj.set_flag(PhysicsObject::Flag::NOGRAVITY);
				phobj.vspeed = DROPSPEED;
				phobj.hspeed = static_cast<double>(dest.x() - start.x()) / air_ticks(dest.y() - start.y() + 4);
				break;
			case 2:
				state = Drop::State::FLOATING;
				basey = phobj.crnt_y();
				phobj.type = PhysicsObject::Type::FIXATED;
				break;
			case 3:
				pick_up();
				break;
		}
	}

	int8_t Drop::update(const Physics &physics)
	{
		if (state == Drop::State::DROPPED)
			phobj.vspeed += DROPGRAVITY;
		else if (state == Drop::State::PICKEDUP)
			phobj.vspeed += PICKUPGRAVITY;

		physics.move_object(phobj);

		if (state == Drop::State::DROPPED)
		{
			if (phobj.onground)
			{
				phobj.hspeed = 0.0;
				phobj.type = PhysicsObject::Type::FIXATED;
				phobj.clear_flag(PhysicsObject::Flag::NOGRAVITY);
				state = Drop::State::FLOATING;
				angle.set(0.0f);
				set_position(dest.x(), dest.y() - 4);
			} else
			{
				// About two turns over the 1.1 s flight, as before the
				// flight was slowed down.
				static const float SPINSTEP = 0.1f;
				angle += SPINSTEP;
			}
		}

		if (state == Drop::State::FLOATING)
		{
			phobj.y = basey + 5.0f + (cos(moved) - 1.0f) * 2.5f;
			moved = (moved < 360.0f) ? moved + 0.025f : 0.0f;
		}

		if (state == Drop::State::PICKEDUP)
		{
			if (looter)
			{
				double hdelta = looter->x - phobj.x;
				phobj.hspeed = looter->hspeed / 2.0 + (hdelta - 16.0) / PICKUPTIME;
			}

			pickupticks++;

			if (pickupticks > PICKUPTIME - PICKUPFADETIME)
				opacity -= 1.0f / PICKUPFADETIME;

			if (pickupticks >= PICKUPTIME)
			{
				opacity.set(1.0f);

				MapObject::deactivate();
				return -1;
			}
		}

		if (state == Drop::State::EXPIRING)
		{
			static const float OPCSTEP = 1.0f / 48;

			opacity -= OPCSTEP;

			if (opacity.last() <= OPCSTEP)
			{
				opacity.set(1.0f);

				MapObject::deactivate();
				return -1;
			}
		}

		return phobj.fhlayer;
	}

	void Drop::expire(int8_t type, const PhysicsObject *lt)
	{
		switch (type)
		{
			case 0:
				phobj.clear_flag(PhysicsObject::Flag::NOGRAVITY);
				state = Drop::State::EXPIRING;
				break;
			case 1:
				deactivate();
				break;
			case 2:
				looter = lt;
				pick_up();
				break;
		}
	}

	void Drop::pick_up()
	{
		angle.set(0.0f);
		state = Drop::State::PICKEDUP;
		pickupticks = 0;
		phobj.hspeed = 0.0;
		phobj.vspeed = PICKUPSPEED;
		phobj.type = PhysicsObject::Type::NORMAL;
		phobj.set_flag(PhysicsObject::Flag::NOGRAVITY);
	}

	Rectangle<int16_t> Drop::bounds() const
	{
		auto lt = get_position();
		auto rb = lt + Point<int16_t>(32, 32);

		return Rectangle<int16_t>(lt, rb);
	}
}