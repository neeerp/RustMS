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

#include <cstdint>

#include "../UIDragElement.h"

#include "../../Character/CharStats.h"
#include "../../Gameplay/MapleMap/MapTooltipRules.h"
#include "../../Template/BoolPair.h"

namespace ms
{
	class UIStatsInfo : public UIDragElement<PosSTATS>
	{
	public:
		static constexpr Type TYPE = UIElement::Type::STATSINFO;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = true;

		UIStatsInfo(const CharStats &stats);

		void draw(float alpha) const override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;
		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;
		void remove_cursor() override;

		bool is_in_range(Point<int16_t> cursorpos) const override;

		UIElement::Type get_type() const override;

		void update_all_stats();
		void update_stat(MapleStat::Id stat);

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		enum StatLabel
		{
			// Main
			NAME,
			LEVEL,
			GUILD,
			HP,
			MP,
			EXP,
			FAME,
			AP,
			STR,
			DEX,
			INT,
			LUK,
			NUM_MAIN,
			// Detail
			ATTACK = NUM_MAIN,
			WDEF,
			MAGIC,
			MDEF,
			ACCURACY,
			AVOID,
			HANDS,
			SPEED,
			JUMP,
			NUM_LABELS
		};

		enum Buttons
		{
			BT_CLOSE,
			BT_HP,
			BT_MP,
			BT_STR,
			BT_DEX,
			BT_INT,
			BT_LUK,
			BT_DETAIL,
			BT_HIDE,
			BT_AUTO,
			// The first-job Pirate's halves: Brawler and Gunslinger.
			BT_AUTO1,
			BT_AUTO2
		};

		void update_ap();

		void update_job();

		void update_basevstotal(StatLabel label, MapleStat::Id bstat, EquipStat::Id tstat);

		void update_buffed(StatLabel label, EquipStat::Id stat, int32_t extra);

		void send_apup(MapleStat::Id stat) const;
		// Ask before spending AP on MaxHP or MaxMP, as v83 does.
		void confirm_apup(MapleStat::Id stat) const;
		// Spread the AP as the job builds, once the player says yes (0x8c8692).
		void auto_assign(bool brawler) const;
		// Whether the cursor is on a shown auto button, whose own help shows instead of the zone's.
		bool on_auto_button(Point<int16_t> cursor) const;

		void set_detail(bool enabled);

		bool is_beginner() const;

		void show_zone_help(Point<int16_t> cursor);
		void clear_zone_help();

		const CharStats &stats;

		Texture detail;
		// Grey STR, DEX, INT and LUK labels, drawn over the ones a beginner's
		// stats don't grow by.
		Texture disabled[4];
		Texture job_main;
		Texture job_sub;
		Text job_name;

		bool showdetail;

		Text statlabels[StatLabel::NUM_LABELS];

		// The help for the window's labels and the detail panel's.
		std::vector<map_tooltips::HelpZone> stat_help;
		std::vector<map_tooltips::HelpZone> detail_help;
		bool zone_help = false;
	};
}
