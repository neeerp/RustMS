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
#include "Bullet.h"

#include "../../Constants.h"

#include <algorithm>
#include <cmath>

namespace ms
{
	Bullet::Bullet(Animation a, Point<int16_t> o, Point<int16_t> t, uint16_t d) : animation(a), origin(o), target(t),
																				   duration(d), elapsed(0)
	{
		flip = target.x() > origin.x();
	}

	void Bullet::draw(double viewx, double viewy, float alpha) const
	{
		Point<double> position = get_position(alpha);
		Point<int16_t> bulletpos(
				static_cast<int16_t>(std::round(position.x() + viewx)),
				static_cast<int16_t>(std::round(position.y() + viewy))
		);

		DrawArgument args(bulletpos, flip);
		animation.draw(args, alpha);
	}

	bool Bullet::update()
	{
		animation.update();
		elapsed = static_cast<uint16_t>(std::min<int32_t>(elapsed + Constants::TIMESTEP, duration));

		return elapsed >= duration;
	}

	Point<double> Bullet::get_position(float alpha) const
	{
		if (duration == 0)
			return { static_cast<double>(target.x()), static_cast<double>(target.y()) };

		double time = elapsed - Constants::TIMESTEP * (1.0 - alpha);
		double progress = std::clamp(time / duration, 0.0, 1.0);

		return {
				origin.x() + (target.x() - origin.x()) * progress,
				origin.y() + (target.y() - origin.y()) * progress
		};
	}
}
