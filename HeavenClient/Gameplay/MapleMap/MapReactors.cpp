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
#include "MapReactors.h"

namespace ms
{
	void MapReactors::draw(Layer::Id layer, double viewx, double viewy, float alpha) const
	{
		reactors.draw(layer, viewx, viewy, alpha);
	}

	// Spawns all reactors to map with proper footholds
	void MapReactors::update(const Physics &physics)
	{
		for (; !spawns.empty(); spawns.pop())
		{
			const ReactorSpawn &spawn = spawns.front();

			int32_t oid = spawn.get_oid();

			if (auto reactor = reactors.get(oid))
				reactor->makeactive();
			else
				reactors.add(spawn.instantiate(physics));
		}

		reactors.update(physics);
	}

	void MapReactors::hit(int32_t oid, int8_t state, uint16_t delay, int8_t event, int32_t cooldown)
	{
		if (Optional<Reactor> reactor = reactors.get(oid))
			reactor->hit(state, delay, event, cooldown);
	}

	void MapReactors::spawn(ReactorSpawn &&spawn)
	{
		spawns.emplace(std::move(spawn));
	}

	void MapReactors::remove(int32_t oid, int8_t state)
	{
		if (Optional<Reactor> reactor = reactors.get(oid))
			reactor->destroy(state);
	}

	std::vector<reactor_rules::TouchChange> MapReactors::touch(Point<int16_t> user)
	{
		std::vector<reactor_rules::Touchable> touchable;

		for (auto &mmo : reactors)
			if (mmo.second)
				touchable.push_back({ mmo.first, static_cast<Reactor *>(mmo.second.get())->get_touch_box() });

		return reactor_rules::touch_changes(touchable, user.x(), user.y(), touched);
	}

	void MapReactors::clear()
	{
		reactors.clear();
		touched.clear();
	}

	MapObjects *MapReactors::get_reactors()
	{
		return &reactors;
	}
}