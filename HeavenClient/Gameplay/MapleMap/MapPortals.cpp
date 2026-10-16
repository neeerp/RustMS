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

#include "../../Constants.h"

#include "../../Util/Misc.h"
#include "../../Net/Packets/GameplayPackets.h"

#include <nlnx/nx.hpp>

namespace ms
{
	MapPortals::MapPortals(nl::node src, int32_t mapid)
	{
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

			const Animation *animation = &animations[type];
			bool intramap = target_id == mapid;

			portals_by_id.emplace(
					std::piecewise_construct,
					std::forward_as_tuple(portal_id),
					std::forward_as_tuple(animation, type, name, intramap, position, target_id, target_name, hrange,
										  vrange)
			);

			portal_ids_by_name.emplace(name, portal_id);

			if (type == Portal::HIDDEN || type == Portal::SCRIPTED_HIDDEN)
			{
				std::string image = sub["image"];

				if (image.empty())
					image = "default";

				nl::node game = nl::nx::map["MapHelper.img"]["portal"]["game"];
				hidden_images.emplace(portal_id, game[type == Portal::HIDDEN ? "ph" : "psh"][image]);
			}
		}

		cooldown = WARPCD;
	}

	MapPortals::MapPortals()
	{
		cooldown = WARPCD;
	}

	void MapPortals::update(Point<int16_t> playerpos)
	{
		animations[Portal::REGULAR].update(Constants::TIMESTEP);

		for (auto &iter : portals_by_id)
		{
			Portal &portal = iter.second;
			switch (portal.get_type())
			{
				case Portal::SCRIPTED_TOUCH:
					// Tutorial hints and other touch scripts run on the server.
					if (portal.touch(playerpos))
						ChangeMapSpecialPacket(portal.get_name()).dispatch();
					break;
				default:
					break;
			}
		}

		update_hidden(playerpos);

		if (cooldown > 0)
			cooldown--;
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

	Point<int16_t> MapPortals::get_portal_by_name(const std::string &portal_name) const
	{
		auto iter = portal_ids_by_name.find(portal_name);

		if (iter != portal_ids_by_name.end())
			return get_portal_by_id(iter->second);
		else
			return {};
	}

	Portal::WarpInfo MapPortals::find_warp_at(Point<int16_t> playerpos)
	{
		if (cooldown == 0)
		{
			cooldown = WARPCD;

			// v83's Up key (FUN_0094df9b, FUN_00712ab1) searches from the
			// last portal to the first, so the highest id wins where two
			// overlap.
			const Portal *found = nullptr;
			uint8_t found_id = 0;

			for (auto &iter : portals_by_id)
			{
				const Portal &portal = iter.second;

				// Spawn points and tutorial triggers can overlap a real exit.
				// They have no travel destination and must not mask that exit.
				if (portal.getwarpinfo().valid && portal.warp_bounds().contains(playerpos) &&
					(!found || iter.first > found_id))
				{
					found = &portal;
					found_id = iter.first;
				}
			}

			if (found)
				return found->getwarpinfo();
		}

		return {};
	}

	void MapPortals::init()
	{
		nl::node src = nl::nx::map["MapHelper.img"]["portal"]["game"];

		animations[Portal::REGULAR] = src["pv"];
	}

	std::unordered_map<Portal::Type, Animation> MapPortals::animations;
}
