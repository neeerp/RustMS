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

#include <array>
#include <string>
#include <vector>

#include <cstdint>

#include "Charset.h"
#include "Tooltip.h"
#include "TooltipBox.h"

#include "../Character/MapleStat.h"
#include "../Graphics/Text.h"
#include "../Template/BoolPair.h"

namespace ms
{
	// The v83 equip tooltip: the name, the icon with the required stats beside
	// it, the jobs that can wear the equip, and a line for each of its stats.
	class EquipTooltip : public Tooltip
	{
	public:
		EquipTooltip();

		void set_equip(Tooltip::Parent parent, int16_t invpos);

		void draw(Point<int16_t> position) const override;

	private:
		// In the order of the job row.
		static constexpr size_t NUM_JOBS = 6;

		struct StatLine
		{
			Text label;
			Text value;
			int16_t gap;
		};

		Tooltip::Parent parent;
		int16_t invpos;
		Point<int16_t> size;

		TooltipBox box;
		TooltipIcon icon;
		ColorBox line;
		Texture stat_dot;

		Text name;
		// The orange lines under the name; the first wraps where it is wider than the box.
		std::array<Text, 2> properties;
		bool wrapped;
		int16_t properties_height;
		Text expiry;
		Texture itemicon;

		std::vector<MapleStat::Id> requirements;
		EnumMap<MapleStat::Id, BoolPair<Texture>> reqlabels;
		EnumMap<MapleStat::Id, bool> canequip;
		EnumMap<MapleStat::Id, std::string> reqvalues;
		BoolPair<Charset> reqdigits;
		BoolPair<Texture> reqnone;

		std::array<BoolPair<Texture>, NUM_JOBS> jobs;
		std::array<bool, NUM_JOBS> okjobs;

		std::vector<StatLine> stats;
		Text desc;
	};
}
