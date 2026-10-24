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
#include "MapInfo.h"

#include "FieldLimit.h"

#include "../Constants.h"

#include <algorithm>
#include <charconv>

namespace ms
{
	MapInfo::MapInfo(nl::node info, nl::node src, Range<int16_t> walls, Range<int16_t> borders)
	{
		// v83 (FUN_005a768c) reads each edge of the view range on its own.
		// One the map leaves out lies 20 px beyond the movement bounds at the
		// sides, 60 px above them, and 110 px below the lowest foothold.
		auto edge = [&info](const char *name, int16_t fallback)
		{
			nl::node value = info[name];

			return value.data_type() == nl::node::type::integer ? static_cast<int16_t>(value) : fallback;
		};

		mapwalls = Range<int16_t>(edge("VRLeft", walls.first() - 20), edge("VRRight", walls.second() + 20));
		mapborders = Range<int16_t>(edge("VRTop", borders.first() - 60), edge("VRBottom", borders.second() + 10));

		std::string bgmpath = info["bgm"];
		size_t split = bgmpath.find('/');

		if (split != std::string::npos)
			bgm = bgmpath.substr(0, split) + ".img/" + bgmpath.substr(split + 1);

		cloud = info["cloud"].get_bool();
		fieldlimit = info["fieldLimit"];
		fieldtype = info["fieldType"];

		for (auto item : info["allowedItem"])
			allowed_items.push_back(item);

		hideminimap = info["hideMinimap"].get_bool();
		mapmark = std::string(info["mapMark"]);
		swim = info["swim"].get_bool();
		fly = info["fly"].get_bool();

		for (auto area : src["swimArea"])
			swim_areas.push_back({ area["x1"], area["x2"], area["y1"], area["y2"] });

		for (auto job : src["noSkill"]["class"])
			no_skills.classes.push_back(job);

		for (auto skill : src["noSkill"]["skill"])
			no_skills.skills.push_back(skill);
		town = info["town"].get_bool();
		personal_shop = info["personalShop"].get_bool();
		everlast = info["everlast"].get_bool();
		recovery = info["recovery"].get_real(1.0);

		for (auto seat : src["seat"])
			seats.push_back(seat);

		// v83 searches ladders and ropes by number, so 2 comes before 10.
		std::vector<nl::node> ropes;

		for (auto rope : src["ladderRope"])
			ropes.push_back(rope);

		std::stable_sort(ropes.begin(), ropes.end(), [](nl::node a, nl::node b) {
			auto number = [](const std::string& name) {
				int32_t value = 0;
				std::from_chars(name.data(), name.data() + name.size(), value);
				return value;
			};
			return number(a.name()) < number(b.name());
		});

		for (auto ladder : ropes)
			ladders.push_back(ladder);
	}

	MapInfo::MapInfo()
	{}

	bool MapInfo::is_flying() const
	{
		return fly && !swim;
	}

	bool MapInfo::is_swimming(Point<int16_t> position) const
	{
		return swim_area::swimming(swim, swim_areas, position.x(), position.y());
	}

	bool MapInfo::forbids_skill(int32_t skill_id) const
	{
		return no_skill::forbids(no_skills, skill_id);
	}

	bool MapInfo::is_town() const
	{
		return town;
	}

	bool MapInfo::allows_personal_shop() const
	{
		return personal_shop;
	}

	bool MapInfo::is_everlast() const
	{
		return everlast;
	}

	int32_t MapInfo::get_fieldlimit() const
	{
		return fieldlimit;
	}

	int32_t MapInfo::get_fieldtype() const
	{
		return fieldtype;
	}

	bool MapInfo::can_migrate() const
	{
		return (fieldlimit & 0x10) == 0;
	}

	bool MapInfo::can_use_movement_skills() const
	{
		return (fieldlimit & field_limit::MOVEMENT_SKILLS) == 0;
	}

	bool MapInfo::can_use_item(int32_t item_id) const
	{
		return field_limit::may_use(fieldlimit, allowed_items, item_id);
	}

	bool MapInfo::can_summon(int32_t item_id) const
	{
		return field_limit::may_summon(fieldlimit, allowed_items, item_id);
	}

	bool MapInfo::can_drop() const
	{
		return field_limit::may_drop(fieldlimit);
	}

	std::string MapInfo::get_bgm() const
	{
		return bgm;
	}

	double MapInfo::get_recovery() const
	{
		return recovery;
	}

	bool MapInfo::has_clouds() const
	{
		return cloud;
	}

	Range<int16_t> MapInfo::get_walls() const
	{
		return mapwalls;
	}

	Range<int16_t> MapInfo::get_borders() const
	{
		return mapborders;
	}

	Optional<const Seat> MapInfo::findseat(Point<int16_t> position) const
	{
		for (auto &seat : seats)
			if (seat.inrange(position))
				return seat;

		return nullptr;
	}

	Optional<const Ladder> MapInfo::findladder(Point<int16_t> a, Point<int16_t> b) const
	{
		auto hor = Range<int16_t>(std::min(a.x(), b.x()) - 10, std::max(a.x(), b.x()) + 10);
		auto ver = Range<int16_t>(std::min(a.y(), b.y()), std::max(a.y(), b.y()));

		for (auto &ladder : ladders)
			if (hor.contains(ladder.get_x()) && ver.overlaps({ladder.get_top(), ladder.get_bottom()}))
				return ladder;

		return nullptr;
	}

	Seat::Seat(nl::node src)
	{
		pos = src;
	}

	bool Seat::inrange(Point<int16_t> position) const
	{
		auto hor = Range<int16_t>::symmetric(position.x(), 10);
		auto ver = Range<int16_t>::symmetric(position.y(), 10);

		return hor.contains(pos.x()) && ver.contains(pos.y());
	}

	Point<int16_t> Seat::getpos() const
	{
		return pos;
	}

	Ladder::Ladder(nl::node src)
	{
		std::string name = src.name();
		id = 0;
		std::from_chars(name.data(), name.data() + name.size(), id);
		x = src["x"];
		y1 = src["y1"];
		y2 = src["y2"];
		ladder = src["l"].get_bool();
		uf = src["uf"].get_bool();
	}

	int16_t Ladder::get_id() const
	{
		return id;
	}

	bool Ladder::is_ladder() const
	{
		return ladder;
	}

	bool Ladder::leads_up() const
	{
		return uf;
	}

	int16_t Ladder::get_x() const
	{
		return x;
	}

	int16_t Ladder::get_top() const
	{
		return y1;
	}

	int16_t Ladder::get_bottom() const
	{
		return y2;
	}
}
