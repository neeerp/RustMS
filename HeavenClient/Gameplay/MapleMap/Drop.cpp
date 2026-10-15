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

#include "../../Audio/Audio.h"

#include <algorithm>
#include <chrono>
#include <cmath>

namespace ms
{
	namespace
	{
		const double PI = 3.14159265358979323846;

		// A drop flies up at 400 px/s (720 when it explodes out of a boss)
		// against 800 px/s^2 of gravity, so a normal drop rises 100 px and
		// comes back to its start height after one second.
		const double GRAVITY = 400.0;
		const double DROPSPEED = 400.0;
		const double EXPLOSIVESPEED = 720.0;

		// Item drops spin once every 300 ms while they fly.
		const double SPINSTEP = 2.0 * PI * Constants::TIMESTEP / 300.0;

		// A lying drop bobs 3 px up and down every two seconds.
		const double FLOATSTEP = PI * Constants::TIMESTEP / 1000.0;
		const double FLOATHEIGHT = 3.0;

		// A picked up drop arcs 40 px above the line to the looter's middle
		// in 700 ms and fades to a quarter over the last 280 ms.
		const int32_t PICKUPTIME = 700;
		const int32_t PICKUPFADESTART = 420;
		const double PICKUPARC = 40.0;
		const int16_t LOOTERHALFHEIGHT = 32;

		const int32_t FADETIME = 1000;
		const int32_t DROPSOUNDGAP = 300;
		const int8_t EXPLOSIVE = 3;

		// Milliseconds the flight lasts. The drop comes back to its start height
		// at the end, except when it lands higher up: then the flight is cut
		// short when it should reach that height.
		int32_t flight_time(int16_t starty, int16_t endy, bool explosive)
		{
			double height = explosive ? 180.0 : 100.0;
			double scale = explosive ? 1.8 : 1.0;
			double longest = 1000.0 * scale;

			if (endy >= starty)
				return static_cast<int32_t>(longest);

			double rise = std::max(endy + height - starty, 0.0);
			double time = 30.0 * (static_cast<int32_t>(std::sqrt(rise * 2.5)) + 1) + 500.0 * scale;

			return static_cast<int32_t>(std::min(time, longest));
		}

		void play_dropsound()
		{
			using Clock = std::chrono::steady_clock;
			static Clock::time_point last;

			Clock::time_point now = Clock::now();

			if (now - last > std::chrono::milliseconds(DROPSOUNDGAP))
			{
				last = now;
				Sound(Sound::Name::DROP).play();
			}
		}
	}

	Drop::Drop(int32_t id, int32_t own, Point<int16_t> strt, Point<int16_t> dst, int8_t type, int8_t md, int16_t dly,
			   bool pldrp, bool spn) : MapObject(id)
	{
		owner = own;
		pickuptype = type;
		mode = md;
		playerdrop = pldrp;
		spins = spn;
		start = {strt.x(), static_cast<int16_t>(strt.y() - 4)};
		dest = {dst.x(), static_cast<int16_t>(dst.y() - 4)};

		phobj.type = PhysicsObject::Type::FIXATED;
		angle.set(0.0f);
		opacity.set(1.0f);
		elapsed = 0;
		delay = std::max<int32_t>(dly, 0);
		flighttime = flight_time(start.y(), dest.y(), pickuptype == EXPLOSIVE);
		phase = 0.0;
		looter = nullptr;

		if (mode == 2)
		{
			state = Drop::State::FLOATING;
			set_position(dest.x(), dest.y());
		} else
		{
			// Hidden until the server's delay has passed.
			state = Drop::State::WAITING;
			opacity.set(0.0f);
			set_position(start.x(), start.y());
		}
	}

	int8_t Drop::update(const Physics &physics)
	{
		elapsed += Constants::TIMESTEP;

		if (state == Drop::State::WAITING)
		{
			if (elapsed < delay)
				return phobj.fhlayer;

			state = Drop::State::DROPPED;
			elapsed = Constants::TIMESTEP;
			opacity.set(1.0f);

			if (mode == 1)
				play_dropsound();
		}

		// These drops only show their flight, fading out as they go.
		if (mode == 3 && (state == Drop::State::DROPPED || state == Drop::State::FALLING))
			opacity -= static_cast<float>(Constants::TIMESTEP) / FADETIME;

		double speed = pickuptype == EXPLOSIVE ? EXPLOSIVESPEED : DROPSPEED;

		if (state == Drop::State::DROPPED)
		{
			if (elapsed < flighttime)
			{
				// Half the way across in the first half second, the rest
				// until the flight ends.
				double rest = elapsed > 500 ? std::min((elapsed - 500.0) / (flighttime - 500.0), 1.0) : 0.0;
				double first = std::min(elapsed / 500.0, 1.0);
				double seconds = elapsed / 1000.0;

				double x = start.x() + (first + rest) * (dest.x() - start.x()) / 2.0;
				double y = start.y() - speed * seconds + GRAVITY * seconds * seconds;

				move_to(physics, std::round(x), std::round(y));

				if (spins)
					angle += static_cast<float>(SPINSTEP);

				return phobj.fhlayer;
			}

			if (start.y() < dest.y())
			{
				state = Drop::State::FALLING;
				elapsed = 0;
			} else
			{
				state = Drop::State::FLOATING;
			}
		}

		if (state == Drop::State::FALLING)
		{
			// Back at its start height, the drop falls the rest of the way
			// at its launch speed.
			double y = start.y() + speed * elapsed / 1000.0;

			if (y < dest.y())
			{
				move_to(physics, dest.x(), std::round(y));

				if (spins)
					angle += static_cast<float>(SPINSTEP);

				return phobj.fhlayer;
			}

			state = Drop::State::FLOATING;
		}

		if (state == Drop::State::FLOATING)
		{
			if (mode == 0 || mode == 3)
			{
				MapObject::deactivate();
				return -1;
			}

			angle.set(0.0f);
			move_to(physics, dest.x(), std::round(dest.y() + FLOATHEIGHT * std::sin(phase)));
			phase += FLOATSTEP;
		}

		if (state == Drop::State::PICKEDUP)
		{
			if (!looter || elapsed >= PICKUPTIME)
			{
				MapObject::deactivate();
				return -1;
			}

			double t = elapsed;
			double targetx = looter->get_x();
			double targety = looter->get_y() - 4 - LOOTERHALFHEIGHT;
			double x = ((PICKUPTIME - t) * pickupstart.x() + t * targetx) / PICKUPTIME;
			double y = ((PICKUPTIME - t) * pickupstart.y() + t * targety) / PICKUPTIME
				- PICKUPARC + PICKUPARC * (t - PICKUPTIME / 2) * (t - PICKUPTIME / 2) / (PICKUPTIME * PICKUPTIME / 4);

			move_to(physics, std::round(x), std::round(y));

			if (elapsed > PICKUPFADESTART)
				opacity = 1.0f - 0.75f * (elapsed - PICKUPFADESTART) / (PICKUPTIME - PICKUPFADESTART);
		}

		if (state == Drop::State::EXPIRING)
		{
			opacity -= static_cast<float>(Constants::TIMESTEP) / FADETIME;

			if (opacity.get() <= 0.0f)
			{
				MapObject::deactivate();
				return -1;
			}
		}

		return phobj.fhlayer;
	}

	void Drop::move_to(const Physics &physics, double x, double y)
	{
		phobj.hspeed = x - phobj.x.get();
		phobj.vspeed = y - phobj.y.get();

		physics.move_object(phobj);
	}

	void Drop::expire(int8_t type, const PhysicsObject *lt)
	{
		switch (type)
		{
			case 0:
				angle.set(0.0f);
				state = Drop::State::EXPIRING;
				break;
			case 1:
				deactivate();
				break;
			default:
				looter = lt;
				pick_up();
				break;
		}
	}

	void Drop::pick_up()
	{
		angle.set(0.0f);
		opacity.set(1.0f);
		state = Drop::State::PICKEDUP;
		elapsed = 0;
		pickupstart = phobj.get_position();
	}

	Point<int16_t> Drop::centred(Point<int16_t> position, Point<int16_t> origin, Point<int16_t> dimensions)
	{
		return position + Point<int16_t>(origin.x() - dimensions.x() / 2, 0);
	}

	Rectangle<int16_t> Drop::bounds() const
	{
		auto lt = get_position() - Point<int16_t>(16, 0);
		auto rb = lt + Point<int16_t>(32, 32);

		return Rectangle<int16_t>(lt, rb);
	}
}
