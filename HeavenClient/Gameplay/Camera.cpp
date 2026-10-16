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
#include "Camera.h"

#include "../Constants.h"

namespace ms
{
	namespace
	{
		// v83 aims the view 50 px above the player's feet.
		constexpr int16_t LOOK_ABOVE = 50;

		// v83 (Shape2D's loose vector, looseness 100, set up by
		// FUN_0042935a) leaves the view still while it is within 28 px of
		// its target on an axis. Further off, it closes the gap d at
		// d² / 100 px/s: over dt ms, 1/d grows by dt / 100000.
		double follow(double current, double target)
		{
			double gap = target - current;
			double distance = std::abs(gap);

			if (distance <= 28.0)
				return current;

			double left = 100000.0 * distance / (Constants::TIMESTEP * distance + 100000.0);

			return gap > 0 ? target - left : target + left;
		}

		// Keep the view inside the map's range, or centre a range smaller
		// than the view (v83 FUN_005a768c).
		double limit(double next, Range<int16_t> bounds, int16_t view)
		{
			if (bounds.length() < view)
				return (bounds.first() + bounds.second() + view) / 2;

			if (next > bounds.first())
				return bounds.first();

			if (next < bounds.second() + view)
				return bounds.second() + view;

			return next;
		}
	}

	Camera::Camera()
	{
		x.set(0.0);
		y.set(0.0);
		lookx = 0.0;
		looky = 0.0;

		VWIDTH = Constants::Constants::get().get_viewwidth();
		VHEIGHT = Constants::Constants::get().get_viewheight();
	}

	void Camera::update(Point<int16_t> position)
	{
		VWIDTH = Constants::Constants::get().get_viewwidth();
		VHEIGHT = Constants::Constants::get().get_viewheight();

		// The followed point isn't held inside the map's range, so the view
		// waits at a wall until the player comes back within 28 px of it.
		lookx = follow(lookx, position.x());
		looky = follow(looky, position.y() - LOOK_ABOVE);

		x = limit(VWIDTH / 2 - lookx, hbounds, VWIDTH);
		y = limit(VHEIGHT / 2 - looky, vbounds, VHEIGHT);
	}

	void Camera::set_position(Point<int16_t> position)
	{
		VWIDTH = Constants::Constants::get().get_viewwidth();
		VHEIGHT = Constants::Constants::get().get_viewheight();

		lookx = position.x();
		looky = position.y() - LOOK_ABOVE;

		x.set(limit(VWIDTH / 2 - lookx, hbounds, VWIDTH));
		y.set(limit(VHEIGHT / 2 - looky, vbounds, VHEIGHT));
	}

	void Camera::set_view(Range<int16_t> mapwalls, Range<int16_t> mapborders)
	{
		hbounds = -mapwalls;
		vbounds = -mapborders;
	}

	Point<int16_t> Camera::position() const
	{
		auto shortx = static_cast<int16_t>(std::round(x.get()));
		auto shorty = static_cast<int16_t>(std::round(y.get()));

		return {shortx, shorty};
	}

	Point<int16_t> Camera::position(float alpha) const
	{
		auto interx = static_cast<int16_t>(std::round(x.get(alpha)));
		auto intery = static_cast<int16_t>(std::round(y.get(alpha)));

		return {interx, intery};
	}

	Point<double> Camera::realposition(float alpha) const
	{
		return {x.get(alpha), y.get(alpha)};
	}
}