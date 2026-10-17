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

#include <string>

#include <cstdint>

#include "Portal.h"

#include <optional>
#include <unordered_map>
#include <vector>

namespace ms
{
	// Collection of portals on a map
	// Draws and updates portals
	// Also contains methods for using portals and obtaining spawn points
	class MapPortals
	{
	public:
		static void init();

		MapPortals(nl::node source, int32_t mapid);

		MapPortals();

		void update(Point<int16_t> playerpos);

		void draw(Point<int16_t> viewpos, float inter) const;

		// The portal the Up key uses where the player stands, if any.
		const Portal *find_up_portal(Point<int16_t> playerpos) const;

		// The touch, touch script or jump portal the player stands in, if any.
		const Portal *find_touch_portal(Point<int16_t> playerpos) const;

		// Whether a touch script may run: one that runs only once must wait
		// until another has run since (v83 FUN_0094dac6).
		bool may_run_script(const Portal &portal) const;

		void ran_script(const Portal &portal);

		Point<int16_t> get_portal_by_id(uint8_t id) const;

		// Where the `i`th of the town's door portals (type 6) stands, in id
		// order, or the first if there are fewer (FUN_005377fa).
		std::optional<Point<int16_t>> get_door_portal(size_t i) const;

		// Whether a portal keeps a Mystic Door from opening where the player
		// stands (FUN_00712ab1).
		bool blocks_door(Point<int16_t> playerpos) const;

		Point<int16_t> get_portal_by_name(const std::string &name) const;

	private:
		void update_hidden(Point<int16_t> playerpos);

		static std::unordered_map<Portal::Type, Animation> animations;

		// Each hidden portal's ph or psh image, which holds its portalStart,
		// portalContinue and portalExit.
		std::unordered_map<uint8_t, nl::node> hidden_images;

		// The one hidden portal showing, and one still playing its exit.
		int16_t shown = -1;
		Animation shown_animation;
		bool shown_continues = false;
		int16_t hiding = -1;
		Animation hiding_animation;

		std::unordered_map<uint8_t, Portal> portals_by_id;
		std::unordered_map<std::string, uint8_t> portal_ids_by_name;
		// The door portals, in id order.
		std::vector<Point<int16_t>> door_portals;

		// The touch script that ran last in this field.
		const Portal *last_script = nullptr;
	};
}