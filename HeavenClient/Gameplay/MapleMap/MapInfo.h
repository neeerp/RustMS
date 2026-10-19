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

#include "../../Template/Optional.h"
#include "../../Template/Point.h"
#include "../../Template/Range.h"

#include <vector>

namespace ms
{
	class Seat
	{
	public:
		Seat(nl::node source);

		bool inrange(Point<int16_t> position) const;

		Point<int16_t> getpos() const;

	private:
		Point<int16_t> pos;
	};

	class Ladder
	{
	public:
		Ladder(nl::node source);

		bool is_ladder() const;

		// Its number in the map.
		int16_t get_id() const;

		// Whether climbing past the top steps onto a platform (uf).
		bool leads_up() const;

		int16_t get_x() const;
		int16_t get_top() const;
		int16_t get_bottom() const;

	private:
		int16_t id;
		int16_t x;
		int16_t y1;
		int16_t y2;
		bool ladder;
		bool uf;
	};

	class MapInfo
	{
	public:
		MapInfo(nl::node src, Range<int16_t> walls, Range<int16_t> borders);

		MapInfo();

		bool is_underwater() const;

		// Whether the map is a town (info/town).
		bool is_town() const;

		// Whether players may open a personal shop here (info/personalShop,
		// the Free Market rooms).
		bool allows_personal_shop() const;

		// The map's fieldLimit bits.
		int32_t get_fieldlimit() const;

		// The map's info/fieldType.
		int32_t get_fieldtype() const;

		std::string get_bgm() const;

		// How much faster players recover HP and MP at rest (info/recovery).
		double get_recovery() const;

		// Whether clouds drift over the map (info/cloud).
		bool has_clouds() const;

		// Whether players may leave the map for another channel or the Cash
		// Shop: fieldLimit bit 0x10 is unset.
		bool can_migrate() const;

		// Whether the map lets the player use Teleport (fieldLimit bit 0x2).
		bool can_use_movement_skills() const;

		// Whether the map lets the player use a potion or the like
		// (field_limit::may_use).
		bool can_use_item(int32_t item_id) const;

		// Whether the map lets the player open a summoning sack
		// (field_limit::may_summon).
		bool can_summon(int32_t item_id) const;

		// Whether the map lets the player drop items and mesos.
		bool can_drop() const;

		Range<int16_t> get_walls() const;

		Range<int16_t> get_borders() const;

		// Find a seat the player's position
		Optional<const Seat> findseat(Point<int16_t> position) const;
		// The first ladder or rope that the segment from a to b reaches: one
		// within 10 px of it across and overlapping it down (FUN_00a45b03).
		Optional<const Ladder> findladder(Point<int16_t> a, Point<int16_t> b) const;

	private:
		int32_t fieldlimit = 0;
		int32_t fieldtype = 0;
		// The items a map with fieldLimit bit 0x400 still allows.
		std::vector<int32_t> allowed_items;
		bool cloud = false;
		std::string bgm;
		std::string mapdesc;
		std::string mapname;
		std::string streetname;
		std::string mapmark;
		bool swim;
		bool town;
		bool personal_shop = false;
		double recovery = 1.0;
		bool hideminimap;
		Range<int16_t> mapwalls;
		Range<int16_t> mapborders;
		std::vector<Seat> seats;
		std::vector<Ladder> ladders;
	};
}