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
#include "MapPortals.h"

#include "DoorRules.h"

#include "../../Constants.h"

#include "../../Util/Misc.h"

#include <nlnx/nx.hpp>

#include <algorithm>

namespace ms
{
	MapPortals::MapPortals(nl::node src, int32_t mapid)
	{
		std::vector<int8_t> door_ids;

		for (auto sub : src)
		{
			int8_t portal_id = string_conversion::or_default<int8_t>(sub.name(), -1);

			if (portal_id < 0)
				continue;

			Portal::Type type = Portal::typebyid(sub["pt"]);
			std::string name = sub["pn"];
			std::string target_name = sub["tn"];
			int32_t target_id = sub["tm"];
			Point<int16_t> position = {sub["x"], sub["y"]};
			auto hrange = static_cast<int16_t>(sub["hRange"].get_integer(Portal::DEFAULT_RANGE));
			auto vrange = static_cast<int16_t>(sub["vRange"].get_integer(Portal::DEFAULT_RANGE));
			auto delay = static_cast<int32_t>(sub["delay"].get_integer());
			bool only_once = sub["onlyOnce"].get_bool();
			Point<int16_t> impact = {sub["horizontalImpact"], sub["verticalImpact"]};

			// v83 draws pv for regular, type 4 and scripted portals alike
			// (FUN_0071165a, FUN_00712313).
			bool visible = type == Portal::REGULAR || type == Portal::TYPE4 || type == Portal::SCRIPTED;
			const Animation *animation = visible ? &animations[Portal::REGULAR] : nullptr;
			bool intramap = Portal::is_intramap(type, target_id, mapid);

			portals_by_id.emplace(
					std::piecewise_construct,
					std::forward_as_tuple(portal_id),
					std::forward_as_tuple(animation, type, name, intramap, position, target_id, target_name, hrange,
										  vrange, delay, only_once, impact)
			);

			portal_ids_by_name.emplace(name, portal_id);

			if (type == Portal::WARP)
				door_ids.push_back(portal_id);

			if (type == Portal::HIDDEN || type == Portal::SCRIPTED_HIDDEN)
			{
				std::string image = sub["image"];

				if (image.empty())
					image = "default";

				nl::node game = nl::nx::map["MapHelper.img"]["portal"]["game"];
				hidden_images.emplace(portal_id, game[type == Portal::HIDDEN ? "ph" : "psh"][image]);
			}
		}

		// The NX file lists the portals by name, "10" before "2".
		std::sort(door_ids.begin(), door_ids.end());

		for (int8_t id : door_ids)
			door_portals.push_back(portals_by_id.at(id).get_position());
	}

	MapPortals::MapPortals() = default;

	std::optional<Point<int16_t>> MapPortals::get_door_portal(size_t i) const
	{
		if (door_portals.empty())
			return std::nullopt;

		return door_portals[i < door_portals.size() ? i : 0];
	}

	bool MapPortals::blocks_door(Point<int16_t> playerpos) const
	{
		for (const auto &iter : portals_by_id)
			if (door_rules::blocks(iter.second.get_type(), iter.second.get_position(), playerpos))
				return true;

		return false;
	}

	void MapPortals::update(Point<int16_t> playerpos)
	{
		animations[Portal::REGULAR].update(Constants::TIMESTEP);

		update_hidden(playerpos);
	}

	void MapPortals::update_hidden(Point<int16_t> playerpos)
	{
		// v83 shows one hidden portal at a time, searching from the last
		// portal to the first (FUN_00712c57).
		int16_t found = -1;

		for (auto &iter : hidden_images)
		{
			uint8_t id = iter.first;

			if (id > found && portals_by_id.at(id).reveal_bounds().contains(playerpos))
				found = id;
		}

		// The portal left behind plays portalExit once. The new one plays
		// portalStart once, then loops portalContinue (FUN_00712d35,
		// FUN_0071332c).
		if (found != shown)
		{
			if (shown >= 0)
			{
				hiding = shown;
				hiding_animation = hidden_images.at(shown)["portalExit"];
			}

			shown = found;

			if (shown >= 0)
			{
				shown_animation = hidden_images.at(shown)["portalStart"];
				shown_continues = false;
			}
		}

		if (shown >= 0 && shown_animation.update() && !shown_continues)
		{
			shown_animation = hidden_images.at(shown)["portalContinue"];
			shown_continues = true;
		}

		if (hiding >= 0 && hiding_animation.update())
			hiding = -1;
	}

	void MapPortals::draw(Point<int16_t> viewpos, float inter) const
	{
		for (auto &ptit : portals_by_id)
			ptit.second.draw(viewpos, inter);

		if (hiding >= 0)
			hiding_animation.draw(portals_by_id.at(hiding).get_position() + viewpos, inter);

		if (shown >= 0)
			shown_animation.draw(portals_by_id.at(shown).get_position() + viewpos, inter);
	}

	Point<int16_t> MapPortals::get_portal_by_id(uint8_t portal_id) const
	{
		auto iter = portals_by_id.find(portal_id);

		if (iter != portals_by_id.end())
		{
			constexpr Point<int16_t> ABOVE(0, 30);

			return iter->second.get_position() - ABOVE;
		} else
		{
			return {};
		}
	}

	bool MapPortals::has_portal(uint8_t portal_id) const
	{
		return portals_by_id.count(portal_id) > 0;
	}

	Point<int16_t> MapPortals::get_portal_by_name(const std::string &portal_name) const
	{
		auto iter = portal_ids_by_name.find(portal_name);

		if (iter != portal_ids_by_name.end())
			return get_portal_by_id(iter->second);
		else
			return {};
	}

	const Portal *MapPortals::find_up_portal(Point<int16_t> playerpos) const
	{
		// v83's Up key (FUN_0094df9b, FUN_00712ab1) takes the last portal
		// other than a spawn point whose box holds the player, even one
		// that leads nowhere.
		const Portal *found = nullptr;
		uint8_t found_id = 0;

		for (auto &iter : portals_by_id)
		{
			const Portal &portal = iter.second;

			if (portal.get_type() != Portal::SPAWN && portal.warp_bounds().contains(playerpos) &&
				(!found || iter.first > found_id))
			{
				found = &portal;
				found_id = iter.first;
			}
		}

		return found;
	}

	const Portal *MapPortals::find_touch_portal(Point<int16_t> playerpos) const
	{
		// v83 (FUN_00712b61) keeps types 3, 9, 12 and 13 in a list of their
		// own and takes the last one whose hRange by vRange box holds the
		// player.
		const Portal *found = nullptr;
		uint8_t found_id = 0;

		for (auto &iter : portals_by_id)
		{
			const Portal &portal = iter.second;

			switch (portal.get_type())
			{
				case Portal::TOUCH:
				case Portal::SCRIPTED_TOUCH:
				case Portal::SPRING1:
				case Portal::SPRING2:
					if (portal.range_bounds().contains(playerpos) && (!found || iter.first > found_id))
					{
						found = &portal;
						found_id = iter.first;
					}
					break;
				default:
					break;
			}
		}

		return found;
	}

	bool MapPortals::may_run_script(const Portal &portal) const
	{
		return !portal.is_only_once() || &portal != last_script;
	}

	void MapPortals::ran_script(const Portal &portal)
	{
		last_script = &portal;
	}

	void MapPortals::init()
	{
		nl::node src = nl::nx::map["MapHelper.img"]["portal"]["game"];

		animations[Portal::REGULAR] = src["pv"];
	}

	std::unordered_map<Portal::Type, Animation> MapPortals::animations;
}
