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
#include "Obj.h"

#include <nlnx/nx.hpp>

namespace ms
{
	Obj::Obj(nl::node src, int32_t order) : order(order)
	{
		nl::node data = nl::nx::map["Obj"][src["oS"] + ".img"][src["l0"]][src["l1"]][src["l2"]];

		animation = Animation(data);
		pos = Point<int16_t>(src["x"], src["y"]);
		flip = src["f"].get_bool();
		z = src["z"];

		// v83 FUN_0063c741.
		obstacle = data["obstacle"].get_integer() != 0;

		if (obstacle)
		{
			trap.damage = data["damage"];
			trap.mobdamage = data["mobdamage"];
			trap.dir = data["dir"];
			trap.disease = data["disease"];
			trap.level = data["level"];
			trap.target_field = data["targetField"];

			if (data["forceHP"])
				trap.force_hp = data["forceHP"];
		}
	}

	void Obj::update()
	{
		animation.update();
	}

	void Obj::draw(Point<int16_t> viewpos, float inter) const
	{
		animation.draw(DrawArgument(pos + viewpos, flip), inter);
	}

	uint8_t Obj::getz() const
	{
		return z;
	}

	int32_t Obj::get_order() const
	{
		return order;
	}

	bool Obj::is_obstacle() const
	{
		return obstacle;
	}

	const obstacle::Trap &Obj::get_trap() const
	{
		return trap;
	}

	bool Obj::obstacle_box(obstacle::Box &out) const
	{
		Rectangle<int16_t> bounds = animation.get_bounds();
		Point<int16_t> lt = bounds.get_left_top();
		Point<int16_t> rb = bounds.get_right_bottom();

		return obstacle::box(pos.x(), pos.y(), { lt.x(), lt.y() }, { rb.x(), rb.y() }, flip, out);
	}

	obstacle::Vec Obj::get_push() const
	{
		return obstacle::push(trap.dir, flip);
	}
}