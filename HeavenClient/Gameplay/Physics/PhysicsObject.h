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

#include "../../Constants.h"

#include "../../Template/Interpolated.h"
#include "../../Template/Point.h"

namespace ms
{
	// Structure that contains all properties for movement calculations
	struct MovingObject
	{
		Linear<double> x;
		Linear<double> y;
		double hspeed = 0.0;
		double vspeed = 0.0;

		void normalize()
		{
			x.normalize();
			y.normalize();
		}

		void move()
		{
			x += hspeed;
			y += vspeed;
		}

		void set_x(double d)
		{
			x.set(d);
		}

		void set_y(double d)
		{
			y.set(d);
		}

		void limitx(double d)
		{
			x = d;
			hspeed = 0.0;
		}

		void limity(double d)
		{
			y = d;
			vspeed = 0.0;
		}

		void movexuntil(double d, uint16_t delay)
		{
			if (delay)
			{
				double hdelta = d - x.get();
				hspeed = Constants::TIMESTEP * hdelta / delay;
			}
		}

		void moveyuntil(double d, uint16_t delay)
		{
			if (delay)
			{
				double vdelta = d - y.get();
				vspeed = Constants::TIMESTEP * vdelta / delay;
			}
		}

		bool hmobile() const
		{
			return hspeed != 0.0;
		}

		bool vmobile() const
		{
			return vspeed != 0.0;
		}

		bool mobile() const
		{
			return hmobile() || vmobile();
		}

		double crnt_x() const
		{
			return x.get();
		}

		double crnt_y() const
		{
			return y.get();
		}

		double next_x() const
		{
			return x + hspeed;
		}

		double next_y() const
		{
			return y + vspeed;
		}

		int16_t get_x() const
		{
			double rounded = std::round(x.get());
			return static_cast<int16_t>(rounded);
		}

		int16_t get_y() const
		{
			double rounded = std::round(y.get());
			return static_cast<int16_t>(rounded);
		}

		int16_t get_last_x() const
		{
			double rounded = std::round(x.last());
			return static_cast<int16_t>(rounded);
		}

		int16_t get_last_y() const
		{
			double rounded = std::round(y.last());
			return static_cast<int16_t>(rounded);
		}

		Point<int16_t> get_position() const
		{
			return {get_x(), get_y()};
		}

		int16_t get_absolute_x(double viewx, float alpha) const
		{
			double interx = x.normalized() ? std::round(x.get()) : x.get(alpha);

			return static_cast<int16_t>(
					std::round(interx + viewx)
			);
		}

		int16_t get_absolute_y(double viewy, float alpha) const
		{
			double intery = y.normalized() ? std::round(y.get()) : y.get(alpha);

			return static_cast<int16_t>(
					std::round(intery + viewy)
			);
		}

		Point<int16_t> get_absolute(double viewx, double viewy, float alpha) const
		{
			return {get_absolute_x(viewx, alpha), get_absolute_y(viewy, alpha)};
		}
	};

	// Structure that contains all properties necessary for physics calculations
	struct PhysicsObject : public MovingObject
	{
		// Determines which physics engine to use
		enum Type
		{
			NORMAL,
			ICE,
			SWIMMING,
			FLYING,
			FIXATED
		};

		enum Flag
		{
			NOGRAVITY = 0x0001,
			TURNATEDGES = 0x0002,
			CHECKBELOW = 0x0004,
			// The owner sets hspeed itself; ground friction leaves it alone.
			NOFRICTION = 0x0008,
			// With TURNATEDGES, stop short of every foothold end that does not
			// lead onto another floor, as v83 does for mobs that walk but
			// cannot jump.
			WALKONLY = 0x0010,
			// With TURNATEDGES, keep within the reach of the current and home
			// foothold groups, and stop to jump at loose ends and rising
			// walls, as v83 does for mobs that walk and jump.
			JUMPER = 0x0020
		};

		Type type = Type::NORMAL;
		int32_t flags = 0;
		uint16_t fhid = 0;
		double fhslope = 0.0;
		int8_t fhlayer = 0;
		double groundbelow = 0.0;
		bool onground = true;
		bool enablejd = false;
		// The platform a down-jump left, which the fall passes through until
		// the object lands somewhere else.
		uint16_t fallstart = 0;

		double hforce = 0.0;
		double vforce = 0.0;
		double hacc = 0.0;
		double vacc = 0.0;

		// Objects wearing a shoe walk and fall by the v83 client's CVecCtrl
		// rules, steered by walkdir (-1, 0, 1) instead of hforce. walkspeed
		// is the shoe's speed multiplier, and walkforce and walkdrag scale
		// its ground acceleration and drag. Everything else keeps
		// HeavenClient's force-and-friction model.
		bool shoe = false;
		double walkspeed = 1.0;
		double walkforce = 1.0;
		double walkdrag = 1.0;
		int8_t walkdir = 0;
		// A knockback queued for the next move, in pixels per second.
		bool impact = false;
		double impactx = 0.0;
		double impacty = 0.0;

		// A shoe with a flyspeed flies through platforms, steered by walkdir
		// and by flydir: -1 to glide down slowly, 0 to dive.
		double flyspeed = 0.0;
		int8_t flydir = 0;

		// A JUMPER's home group, whether it jumps or carries on at the end of
		// its foothold, and whether it walks down a step there rather than
		// jumping. jumprequest is set when it stops at an end to jump.
		uint16_t homegroup = 0;
		bool edgejump = true;
		bool stepdown = true;
		bool jumprequest = false;

		bool is_flag_set(Flag f)
		{
			return (flags & f) != 0;
		}

		bool is_flag_not_set(Flag f)
		{
			return !is_flag_set(f);
		}

		void set_flag(Flag f)
		{
			flags |= f;
		}

		void clear_flag(Flag f)
		{
			flags &= ~f;
		}

		void clear_flags()
		{
			flags = 0;
		}
	};
}