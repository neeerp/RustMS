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
#include "MapObjects.h"

namespace ms
{
	void MapObjects::draw(Layer::Id layer, double viewx, double viewy, float alpha) const
	{
		for (auto &[stack, oid] : layers[layer])
		{
			auto mmo = get(oid);

			if (mmo && mmo->is_active())
				mmo->draw(viewx, viewy, alpha);
		}
	}

	void MapObjects::draw_balloons(double viewx, double viewy, float alpha) const
	{
		for (auto &layer : layers)
		{
			for (auto &[stack, oid] : layer)
			{
				auto mmo = get(oid);

				if (mmo && mmo->is_active())
					mmo->draw_balloons(viewx, viewy, alpha);
			}
		}
	}

	void MapObjects::update(const Physics &physics)
	{
		for (auto iter = objects.begin(); iter != objects.end();)
		{
			bool remove_mob = false;
			int32_t oid = iter->first;

			if (auto &mmo = iter->second)
			{
				int8_t newlayer = mmo->update(physics);

				if (newlayer == -1)
				{
					remove_mob = true;
				} else
				{
					Place current = places.at(oid);
					bool restacked = mmo->take_restack();

					if (restacked || newlayer != current.layer)
					{
						unplace(oid);
						place(oid, newlayer, restacked ? ++highest_stack : current.stack);
					}
				}
			} else
			{
				remove_mob = true;
			}

			if (remove_mob)
			{
				unplace(oid);
				entries.erase(oid);
				iter = objects.erase(iter);
			}
			else
				iter++;
		}
	}

	void MapObjects::clear()
	{
		objects.clear();
		entries.clear();
		places.clear();
		lowest_stack = 0;
		highest_stack = 0;

		for (auto &layer : layers)
			layer.clear();
	}

	bool MapObjects::contains(int32_t oid) const
	{
		return objects.count(oid) > 0;
	}

	void MapObjects::add(std::unique_ptr<MapObject> toadd)
	{
		int32_t oid = toadd->get_oid();
		int8_t layer = toadd->get_layer();
		int32_t stack = toadd->take_restack() ? ++highest_stack : --lowest_stack;
		unplace(oid);
		objects[oid] = std::move(toadd);
		place(oid, layer, stack);
		entries.emplace(oid, next_entry++);
	}

	void MapObjects::remove(int32_t oid)
	{
		auto iter = objects.find(oid);

		if (iter != objects.end() && iter->second)
		{
			objects.erase(iter);
			entries.erase(oid);
			unplace(oid);
		}
	}

	void MapObjects::place(int32_t oid, int8_t layer, int32_t stack)
	{
		layers[layer].emplace(stack, oid);
		places[oid] = { layer, stack };
	}

	void MapObjects::unplace(int32_t oid)
	{
		auto iter = places.find(oid);

		if (iter != places.end())
		{
			layers[iter->second.layer].erase(iter->second.stack);
			places.erase(iter);
		}
	}

	uint32_t MapObjects::entry_order(int32_t oid) const
	{
		auto iter = entries.find(oid);

		return iter != entries.end() ? iter->second : UINT32_MAX;
	}

	Optional<MapObject> MapObjects::get(int32_t oid)
	{
		auto iter = objects.find(oid);

		return iter != objects.end() ? iter->second.get() : nullptr;
	}

	Optional<const MapObject> MapObjects::get(int32_t oid) const
	{
		auto iter = objects.find(oid);

		return iter != objects.end() ? iter->second.get() : nullptr;
	}

	MapObjects::underlying_t::iterator MapObjects::begin()
	{
		return objects.begin();
	}

	MapObjects::underlying_t::iterator MapObjects::end()
	{
		return objects.end();
	}

	MapObjects::underlying_t::const_iterator MapObjects::begin() const
	{
		return objects.begin();
	}

	MapObjects::underlying_t::const_iterator MapObjects::end() const
	{
		return objects.end();
	}

	MapObjects::underlying_t::size_type MapObjects::size() const
	{
		return objects.size();
	}
}