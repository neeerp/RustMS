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
#include "Physics.h"

#include <algorithm>
#include <cmath>

namespace ms
{
	// v83 Map/Physics.img values, as the MapleLegends client loads them
	// (FUN_0085f495), in pixels and seconds. Forces are divided by the 100
	// mass of a CAttrShoe.
	const double WALKACC = 140000.0 / 100.0;
	const double WALKSPEED = 125.0;
	const double WALKDRAG = 80000.0 / 100.0;
	const double SLIPACC = 60000.0 / 100.0;
	const double SLIPSPEED = 120.0;
	const double FLOATDRAG = 10000.0 / 100.0;
	const double FLOATCOEFFICIENT = 0.01;
	const double GRAVITYACC = 2000.0;
	const double FALLSPEED = 670.0;
	const double JUMPSPEED = 555.0;
	const double DOWNJUMPSPEED = JUMPSPEED * 0.35355339;
	// CAttrShoe::walkSlant: slopes steeper than this slide.
	const double WALKSLANT = 0.9;
	// Steering in the air pushes at twice floatDrag2 towards a speed of
	// walkSpeed / walkForce * floatDrag2.
	const double AIRACC = 2.0 * FLOATDRAG;
	const double AIRSPEED = 125.0 / 140000.0 * 10000.0;
	// Flying pushes at flyForce towards flySpeed, and floatDrag1 slows it.
	const double FLYACC = 120000.0 / 100.0;
	const double FLYSPEED = 200.0;
	const double FLYDRAG = 100000.0 / 100.0;
	const double TICK = Constants::TIMESTEP / 1000.0;

	// CVecCtrl's accelerate (FUN_007f29ab): speeds up towards +-limit and
	// stops there, but never slows a faster object.
	void accelerate(double &speed, double acc, double limit)
	{
		if (limit < 0.0)
			return;

		if (acc > 0.0)
		{
			if (speed < limit)
				speed = std::min(speed + acc * TICK, limit);
		} else if (speed > -limit)
		{
			speed = std::max(speed + acc * TICK, -limit);
		}
	}

	// CVecCtrl's ground drag factor: the shoe's walkDrag clamped to
	// Physics.img's minFriction and maxFriction, halved when below 1.
	double ground_drag(double walkdrag)
	{
		double drag = std::clamp(walkdrag, 0.05, 2.0);

		return drag < 1.0 ? drag * 0.5 : drag;
	}

	// Slows an object faster than +-limit back towards it.
	void decelerate(double &speed, double limit, double drag)
	{
		if (speed > limit)
			speed = std::max(speed - drag * TICK, limit);
		else if (speed < -limit)
			speed = std::min(speed + drag * TICK, -limit);
	}

	const double GRAVFORCE = 0.14;
	const double SWIMGRAVFORCE = 0.03;
	const double FRICTION = 0.3;
	const double SLOPEFACTOR = 0.1;
	const double GROUNDSLIP = 3.0;
	const double FLYFRICTION = 0.05;
	const double SWIMFRICTION = 0.08;

	Physics::Physics(nl::node src)
	{
		fht = src;
	}

	Physics::Physics()
	{}

	void Physics::move_object(PhysicsObject &phobj) const
	{
		// A flier never stands on a platform: v83's mobs (FUN_007f9202)
		// fly through them and only keep within the map.
		if (phobj.shoe && phobj.flyspeed > 0.0)
		{
			phobj.onground = false;
			move_shoe(phobj);
			return;
		}

		// Determine which platform the object is currently on
		fht.update_fh(phobj);

		if (phobj.shoe && phobj.type == PhysicsObject::Type::NORMAL)
		{
			move_shoe(phobj);
			return;
		}

		// Use the appropriate physics for the terrain the object is on
		switch (phobj.type)
		{
			case PhysicsObject::Type::NORMAL:
				move_normal(phobj);
				fht.limit_movement(phobj);
				break;
			case PhysicsObject::Type::FLYING:
				move_flying(phobj);
				fht.limit_movement(phobj);
				break;
			case PhysicsObject::Type::SWIMMING:
				move_swimming(phobj);
				fht.limit_movement(phobj);
				break;
			case PhysicsObject::Type::FIXATED:
			default:
				break;
		}

		// Move the object forward
		phobj.move();
	}

	void Physics::move_shoe(PhysicsObject &phobj) const
	{
		double hspeed = phobj.hspeed / TICK;
		double vspeed = phobj.vspeed / TICK;
		int8_t dir = phobj.walkdir;

		// An upward force on the ground is a takeoff speed.
		if (phobj.onground && phobj.vforce < 0.0)
		{
			vspeed = phobj.vforce / TICK;
			phobj.onground = false;
		}

		phobj.hforce = 0.0;
		phobj.vforce = 0.0;
		phobj.walkdir = 0;

		double hstart = hspeed;
		double vstart = vspeed;

		if (phobj.onground && phobj.is_flag_set(PhysicsObject::Flag::NOFRICTION))
		{
			// The owner slows a knocked-back object itself.
			vspeed = 0.0;
		} else if (phobj.onground)
		{
			// CVecCtrl's walking (FUN_007f21d2) works on the speed along the
			// foothold, with uy the foothold's downward slant.
			double ux = 1.0 / std::sqrt(1.0 + phobj.fhslope * phobj.fhslope);
			double uy = phobj.fhslope * ux;
			double slant = std::abs(uy);
			double speed = hspeed / ux;
			double limit = WALKSPEED * phobj.walkspeed;
			double acc = WALKACC * phobj.walkforce;
			double drag = WALKDRAG * ground_drag(phobj.walkdrag);

			if (slant == 0.0)
			{
				decelerate(speed, limit, drag);

				if (dir == 0)
					decelerate(speed, 0.0, drag);
				else
					accelerate(speed, dir * acc, limit);
			} else
			{
				int8_t uphill = uy >= 0.0 ? -1 : 1;
				double steep = slant * slant;
				double force = dir * acc * (uphill < 0 ? 1.0 + steep : 1.0 - steep);
				double downhill = (1.0 + steep) * limit;

				decelerate(speed, uphill * speed > 0.0 ? limit : downhill, drag);

				if (slant <= WALKSLANT)
				{
					if (dir == 0)
						decelerate(speed, 0.0, drag);
					else
						accelerate(speed, force, uphill * force > 0.0 ? limit : downhill);
				} else
				{
					double slip = -uphill * slant * SLIPACC;
					double sliplimit = slant * SLIPSPEED;

					if (dir * uphill > 0)
					{
						slip *= 0.5;
						sliplimit *= 0.5;
					} else if (dir != 0)
					{
						slip += force;
						sliplimit += downhill;
					}

					if (uphill * speed > 0.0)
						decelerate(speed, 0.0, drag);

					accelerate(speed, slip, sliplimit);
				}
			}

			hspeed = speed * ux;
			vspeed = 0.0;
		} else if (phobj.flyspeed > 0.0)
		{
			// CVecCtrl's flying (FUN_007f2a1c) slows anything faster than
			// flySpeed, then pushes the way the object steers.
			double limit = FLYSPEED * phobj.flyspeed;

			decelerate(hspeed, limit, FLYDRAG);

			if (dir == 0)
				decelerate(hspeed, 0.0, FLYDRAG);
			else
				accelerate(hspeed, dir * FLYACC, limit);

			decelerate(vspeed, limit, FLYDRAG);

			if (phobj.flydir < 0)
			{
				// Gliding settles on sinking at 0.3 of flySpeed, slowing a
				// faster fall at the full push and picking up at half.
				double glide = 0.3 * limit;

				if (vspeed > glide)
					vspeed = std::max(vspeed - FLYACC * TICK, glide);
				else
					vspeed = std::min(vspeed + 0.5 * FLYACC * TICK, glide);
			} else
			{
				accelerate(vspeed, FLYACC, limit);
			}
		} else
		{
			// CVecCtrl's falling (FUN_007f2a1c).
			if (vspeed > FALLSPEED)
				vspeed = std::max(vspeed - FLOATDRAG * TICK, FALLSPEED);

			if (phobj.is_flag_not_set(PhysicsObject::Flag::NOGRAVITY))
				accelerate(vspeed, GRAVITYACC, FALLSPEED);

			if (dir != 0)
				accelerate(hspeed, dir * AIRACC, AIRSPEED);
			else
				decelerate(hspeed, 0.0, vspeed < FALLSPEED ? FLOATDRAG * FLOATCOEFFICIENT : FLOATDRAG);
		}

		// Positions advance by the average of the old and new speeds. A
		// collision zeroes the step, and then the speed stays zero.
		phobj.hspeed = (hstart + hspeed) / 2.0 * TICK;
		phobj.vspeed = (vstart + vspeed) / 2.0 * TICK;

		if (phobj.flyspeed > 0.0)
		{
			// The map's bounds stop a flier without slowing it.
			phobj.move();
			fht.limit_flight(phobj);
			phobj.hspeed = hspeed * TICK;
			phobj.vspeed = vspeed * TICK;

			return;
		}

		bool hstep = phobj.hspeed != 0.0;
		bool vstep = phobj.vspeed != 0.0;

		fht.limit_movement(phobj);
		phobj.move();

		if (!hstep || phobj.hspeed != 0.0)
			phobj.hspeed = hspeed * TICK;

		if (!vstep || phobj.vspeed != 0.0)
			phobj.vspeed = vspeed * TICK;
	}

	double Physics::walk_speed(double walkspeed)
	{
		return WALKSPEED * walkspeed * TICK;
	}

	double Physics::jump_speed(double walkjump)
	{
		return JUMPSPEED * walkjump * TICK;
	}

	void Physics::jump(PhysicsObject &phobj, double jumpspeed, int8_t dir)
	{
		phobj.vforce = -jumpspeed;

		if (dir == 0)
			return;

		// CVecCtrl::Jump (FUN_007f1bb0) measures the walking speed along the
		// foothold and raises it by the slant when jumping downhill.
		double ux = 1.0 / std::sqrt(1.0 + phobj.fhslope * phobj.fhslope);
		double uy = phobj.fhslope * ux;
		double limit = walk_speed(phobj.walkspeed) * (1.0 + uy * uy * (dir * uy >= 0.0 ? 1.0 : -1.0));
		double speed = phobj.hspeed / ux;

		if (dir * speed < limit * 0.8)
			speed += dir * limit * 0.8;

		if (dir * speed > limit)
			speed = dir * limit;

		phobj.hspeed = speed * ux;
	}

	// CVecCtrl::Jump (FUN_007f1bb0) for a flier in the air: it rises at
	// its full flySpeed, keeping its sideways speed.
	void Physics::fly_jump(PhysicsObject &phobj)
	{
		phobj.vspeed = -FLYSPEED * phobj.flyspeed * TICK;
	}

	// CVecCtrl's down-jump (FUN_007f1ac6) rises at jumpSpeed / sqrt(8)
	// whatever the shoe, stops moving sideways, and ignores the platform it
	// left until it lands somewhere else.
	void Physics::jump_down(PhysicsObject &phobj)
	{
		phobj.fallstart = phobj.fhid;
		phobj.onground = false;
		phobj.hspeed = 0.0;
		phobj.vspeed = -DOWNJUMPSPEED * TICK;
	}

	void Physics::move_normal(PhysicsObject &phobj) const
	{
		phobj.vacc = 0.0;
		phobj.hacc = 0.0;

		if (phobj.onground)
		{
			phobj.vacc += phobj.vforce;
			phobj.hacc += phobj.hforce;

			if (phobj.is_flag_set(PhysicsObject::Flag::NOFRICTION))
			{
			} else if (phobj.hacc == 0.0 && phobj.hspeed < 0.1 && phobj.hspeed > -0.1)
			{
				phobj.hspeed = 0.0;
			} else
			{
				double inertia = phobj.hspeed / GROUNDSLIP;
				double slopef = phobj.fhslope;

				if (slopef > 0.5)
					slopef = 0.5;
				else if (slopef < -0.5)
					slopef = -0.5;

				phobj.hacc -= (FRICTION + SLOPEFACTOR * (1.0 + slopef * -inertia)) * inertia;
			}
		} else if (phobj.is_flag_not_set(PhysicsObject::Flag::NOGRAVITY))
		{
			phobj.vacc += GRAVFORCE;
		}

		phobj.hforce = 0.0;
		phobj.vforce = 0.0;

		phobj.hspeed += phobj.hacc;
		phobj.vspeed += phobj.vacc;
		// Publish takeoff immediately so the controller enters its airborne
		// state on this tick rather than treating the jump as another walk tick.
		if (phobj.vspeed < 0.0) phobj.onground = false;
	}

	void Physics::move_flying(PhysicsObject &phobj) const
	{
		phobj.hacc = phobj.hforce;
		phobj.vacc = phobj.vforce;
		phobj.hforce = 0.0;
		phobj.vforce = 0.0;

		phobj.hacc -= FLYFRICTION * phobj.hspeed;
		phobj.vacc -= FLYFRICTION * phobj.vspeed;

		phobj.hspeed += phobj.hacc;
		phobj.vspeed += phobj.vacc;

		if (phobj.hacc == 0.0 && phobj.hspeed < 0.1 && phobj.hspeed > -0.1)
			phobj.hspeed = 0.0;

		if (phobj.vacc == 0.0 && phobj.vspeed < 0.1 && phobj.vspeed > -0.1)
			phobj.vspeed = 0.0;
	}

	void Physics::move_swimming(PhysicsObject &phobj) const
	{
		phobj.hacc = phobj.hforce;
		phobj.vacc = phobj.vforce;
		phobj.hforce = 0.0;
		phobj.vforce = 0.0;

		phobj.hacc -= SWIMFRICTION * phobj.hspeed;
		phobj.vacc -= SWIMFRICTION * phobj.vspeed;

		if (phobj.is_flag_not_set(PhysicsObject::Flag::NOGRAVITY))
			phobj.vacc += SWIMGRAVFORCE;

		phobj.hspeed += phobj.hacc;
		phobj.vspeed += phobj.vacc;

		if (phobj.hacc == 0.0 && phobj.hspeed < 0.1 && phobj.hspeed > -0.1)
			phobj.hspeed = 0.0;

		if (phobj.vacc == 0.0 && phobj.vspeed < 0.1 && phobj.vspeed > -0.1)
			phobj.vspeed = 0.0f;
	}

	Point<int16_t> Physics::get_y_below(Point<int16_t> position) const
	{
		int16_t ground = fht.get_y_below(position);

		return Point<int16_t>(position.x(), ground - 1);
	}

	const FootholdTree &Physics::get_fht() const
	{
		return fht;
	}
}
