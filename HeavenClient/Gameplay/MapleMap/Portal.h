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

#include "../Graphics/Animation.h"
#include "../Template/Rectangle.h"

#include <cstdint>
#include <map>

namespace ms
{
	class Portal
	{
	public:
		enum Type
		{
			SPAWN,
			INVISIBLE,
			REGULAR,
			TOUCH,
			TYPE4,
			TYPE5,
			WARP,
			SCRIPTED,
			SCRIPTED_INVISIBLE,
			SCRIPTED_TOUCH,
			HIDDEN,
			SCRIPTED_HIDDEN,
			SPRING1,
			SPRING2,
			TYPE14
		};

		// A portal's hRange and vRange when the map doesn't set them, as in the
		// original's CPortalList::Init.
		static constexpr int16_t DEFAULT_RANGE = 100;

		static Type typebyid(int32_t id)
		{
			return static_cast<Type>(id);
		}

		struct WarpInfo
		{
			int32_t mapid;
			std::string toname;
			std::string name;
			bool intramap;
			bool valid;

			WarpInfo(int32_t m, bool i, std::string tn, std::string n) : mapid(m), intramap(i), toname(tn), name(n)
			{
				valid = mapid < 999999999;
			}

			WarpInfo() : WarpInfo(999999999, false, {}, {})
			{}
		};

		Portal(const Animation *animation, Type type, std::string name, bool intramap, Point<int16_t> position,
			   int32_t tomap, std::string toname, int16_t hrange, int16_t vrange);

		Portal();

		void update(Point<int16_t> playerpos);

		// Track contact; true only on the update the player first touches it.
		bool touch(Point<int16_t> playerpos);

		void draw(Point<int16_t> viewpos, float alpha) const;

		std::string get_name() const;

		Type get_type() const;

		Point<int16_t> get_position() const;

		Rectangle<int16_t> bounds() const;

		// The box v83 checks touch portals against every update
		// (FUN_00712b61, FindPortal_Collision): hRange by vRange centred on
		// the portal, with the right and bottom edges excluded.
		Rectangle<int16_t> range_bounds() const;

		// The box the Up key finds a portal in (v83 FUN_00712ab1): 20 px
		// either side and 50 px above and below, without the right and
		// bottom edges.
		Rectangle<int16_t> warp_bounds() const;

		WarpInfo getwarpinfo() const;

	private:

		const Animation *animation;
		Type type;
		std::string name;
		Point<int16_t> position;
		WarpInfo warpinfo;
		int16_t hrange;
		int16_t vrange;
		bool touched;
	};
}