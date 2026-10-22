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

#include "../Character/Inventory/Equip.h"
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
		// A Cash Shop commodity lasting `period` days, 0 for ever: the equip as Character.nx makes it, with the
		// period for a date (FUN_004baf1a).
		void set_commodity(int32_t item_id, int32_t period);
		// An equip in the Cash Inventory: the equip as Character.nx makes it, with its date and who sent it
		// (FUN_004b0764).
		void set_cash_item(int32_t item_id, int64_t expiration, const std::string &gift_from);
		// An equip that is not the player's, as a packet made it: in storage, a trade, a shop or a parcel.
		void set_stored(Tooltip::Parent parent, const Equip &equip, bool cash);

		void draw(Point<int16_t> position) const override;

	private:
		// In the order of the job row.
		static constexpr size_t NUM_JOBS = 6;

		// `cash_id` is the serial of the player's own equip, which a crush or friendship ring's line needs.
		void build(const Equip &equip, const std::string &expiry, const std::string &gift, int64_t cash_id = 0);

		struct StatLine
		{
			Text label;
			Text value;
			int16_t gap;
		};

		Tooltip::Parent parent;
		int16_t invpos;
		Point<int16_t> size;
		// The box's, at least WIDTH.
		int16_t width;

		TooltipBox box;
		TooltipIcon icon;
		ColorBox line;
		Texture stat_dot;

		// Whose name an Item Tag printed on it, orange with a white "'s" after, over the name.
		Text owner;
		Text owner_s;
		Text name;
		// The orange lines under the name; the first wraps where it is wider than the box.
		std::array<Text, 2> properties;
		bool wrapped;
		int16_t properties_height;
		Text expiry;
		Text gift;
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
