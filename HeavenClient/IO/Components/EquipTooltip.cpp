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
#include "EquipTooltip.h"

#include "../../Character/Inventory/Equip.h"
#include "../../Data/WeaponData.h"
#include "../../Gameplay/Stage.h"
#include "../../Util/NxFiles.h"

namespace ms
{
	namespace
	{
		// Measured from period footage.
		constexpr int16_t WIDTH = 236;
		// The required stats, right of the icon, with their values after them.
		const Point<int16_t> REQ_POS = { 94, 31 };
		constexpr int16_t REQ_STEP = 12;
		constexpr int16_t REQ_VALUE_X = 50;
		constexpr int16_t DIGIT_STEP = 6;
		const Point<int16_t> JOBS_POS = { 10, 108 };
		constexpr int16_t JOB_GAP = 2;
		const Point<int16_t> LINE_POS = { 7, 128 };
		// The stat lines, each after an orange dot.
		// Text draws its capitals 9 below its position: the first line's
		// capitals start at 135, with the dot halfway down them.
		constexpr int16_t STATS_TOP = 126;
		constexpr int16_t STAT_STEP = 13;
		const Point<int16_t> STAT_DOT = { 10, 11 };
		constexpr int16_t STAT_X = 16;
		// Numbers stand a little apart from their labels, words right after.
		constexpr int16_t VALUE_GAP = 4;
		constexpr int16_t DESC_X = 8;
		constexpr int16_t DESC_WIDTH = 220;

		constexpr const char *JOB_NAMES[] = { "beginner", "warrior", "magician", "bowman", "thief", "pirate" };

		// A stat line's label, as in footage.
		std::string stat_label(EquipStat::Id stat)
		{
			switch (stat)
			{
				case EquipStat::Id::STR:
					return "STR";
				case EquipStat::Id::DEX:
					return "DEX";
				case EquipStat::Id::INT:
					return "INT";
				case EquipStat::Id::LUK:
					return "LUK";
				case EquipStat::Id::HP:
					return "MaxHP";
				case EquipStat::Id::MP:
					return "MaxMP";
				case EquipStat::Id::WATK:
					return "WEAPON ATTACK";
				case EquipStat::Id::MAGIC:
					return "MAGIC ATTACK";
				case EquipStat::Id::WDEF:
					return "WEAPON DEF.";
				case EquipStat::Id::MDEF:
					return "MAGIC DEF.";
				case EquipStat::Id::ACC:
					return "ACCURACY";
				case EquipStat::Id::AVOID:
					return "AVOIDABILITY";
				case EquipStat::Id::HANDS:
					return "HANDS";
				case EquipStat::Id::SPEED:
					return "SPEED";
				case EquipStat::Id::JUMP:
					return "JUMP";
				default:
					return "";
			}
		}

		std::string speed_name(uint8_t speed)
		{
			if (speed <= 2)
				return "FASTEST";
			else if (speed == 3)
				return "FASTER";
			else if (speed == 4)
				return "FAST";
			else if (speed <= 6)
				return "NORMAL";
			else if (speed == 7)
				return "SLOW";
			else if (speed == 8)
				return "SLOWER";
			else
				return "SLOWEST";
		}
	}

	EquipTooltip::EquipTooltip() :
		line(WIDTH - 2 * LINE_POS.x(), 1, Color::Name::WHITE, 0.9f)
	{
		nl::node Equip = NxFiles::ui_classic["UIWindow.img"]["ToolTip"]["Equip"];
		nl::node Can = Equip["Can"];
		nl::node Cannot = Equip["Cannot"];

		stat_dot = Equip["Dot"]["1"];

		requirements = {
			MapleStat::Id::LEVEL,
			MapleStat::Id::STR,
			MapleStat::Id::DEX,
			MapleStat::Id::INT,
			MapleStat::Id::LUK,
			MapleStat::Id::FAME
		};

		const char *reqnames[] = { "reqLEV", "reqSTR", "reqDEX", "reqINT", "reqLUK", "reqPOP" };

		for (size_t i = 0; i < requirements.size(); i++)
		{
			reqlabels[requirements[i]][true] = Can[reqnames[i]];
			reqlabels[requirements[i]][false] = Cannot[reqnames[i]];
		}

		reqdigits[true] = Charset(Can, Charset::Alignment::LEFT);
		reqdigits[false] = Charset(Cannot, Charset::Alignment::LEFT);
		reqnone[true] = Can["none"];
		reqnone[false] = Cannot["none"];

		for (size_t i = 0; i < NUM_JOBS; i++)
		{
			jobs[i][true] = Can[JOB_NAMES[i]];
			jobs[i][false] = Cannot[JOB_NAMES[i]];
		}

		parent = Tooltip::Parent::NONE;
		invpos = 0;
	}

	void EquipTooltip::set_equip(Tooltip::Parent p, int16_t ivp)
	{
		if (parent == p && invpos == ivp)
			return;

		parent = p;
		invpos = ivp;

		InventoryType::Id invtype;

		switch (parent)
		{
			case Tooltip::Parent::ITEMINVENTORY:
			case Tooltip::Parent::SHOP:
				invtype = InventoryType::Id::EQUIP;
				break;
			case Tooltip::Parent::EQUIPINVENTORY:
				invtype = InventoryType::Id::EQUIPPED;
				break;
			default:
				invtype = InventoryType::Id::NONE;
		}

		const Player &player = Stage::get().get_player();
		const CharStats &charstats = player.get_stats();
		auto oequip = player.get_inventory().get_equip(invtype, invpos);

		if (!oequip)
		{
			invpos = 0;
			return;
		}

		const Equip &equip = *oequip;
		int32_t item_id = equip.get_item_id();
		const EquipData &equipdata = EquipData::get(item_id);
		const ItemData &itemdata = equipdata.get_itemdata();

		// Footage runs the upgrades and the gender together: "(+4Male)".
		std::string gender;

		switch (itemdata.get_gender())
		{
			case 0:
				gender = "Male";
				break;
			case 1:
				gender = "Female";
				break;
			default:
				break;
		}

		std::string namestr = itemdata.get_name();

		if (equip.get_level() > 0)
			namestr += " (+" + std::to_string(equip.get_level()) + gender + ")";
		else if (!gender.empty())
			namestr += " (" + gender + ")";

		// Footage shows upgraded equips' names in light blue.
		name = Text(Text::Font::A12B, Text::Alignment::LEFT, equip.get_level() > 0 ? Color::Name::CORNFLOWERBLUE : Color::Name::WHITE, namestr);
		itemicon = itemdata.get_icon(false);

		for (MapleStat::Id ms : requirements)
		{
			int16_t req = equipdata.get_reqstat(ms);
			canequip[ms] = charstats.get_stat(ms) >= req;
			reqvalues[ms] = (ms == MapleStat::Id::FAME && req == 0) ? "" : std::to_string(req);
		}

		int16_t reqjob = equipdata.get_reqstat(MapleStat::Id::JOB);

		// The jobs that can wear the equip are orange, whatever the player's.
		// reqJob has a bit per job from warriors on; 0 is anyone.
		for (size_t i = 0; i < NUM_JOBS; i++)
		{
			if (reqjob == 0)
				okjobs[i] = true;
			else if (reqjob < 0)
				okjobs[i] = i == 0;
			else
				okjobs[i] = i > 0 && (reqjob & (1 << (i - 1)));
		}

		stats.clear();

		auto add_line = [&](const std::string &label, const std::string &value, bool word)
		{
			StatLine statline;
			statline.label = Text(Text::Font::A11M, Text::Alignment::LEFT, Color::Name::WHITE, label + " :");
			statline.value = Text(word ? Text::Font::A11M : Text::Font::A11B, Text::Alignment::LEFT,
								  Color::Name::WHITE, value);
			statline.gap = word ? 0 : VALUE_GAP;
			stats.push_back(statline);
		};

		add_line("CATEGORY", equipdata.get_type(), true);

		if (equipdata.is_weapon())
			add_line("ATTACK SPEED", speed_name(WeaponData::get(item_id).get_speed()), true);

		for (EquipStat::Id es = EquipStat::Id::STR; es <= EquipStat::Id::JUMP; es = static_cast<EquipStat::Id>(es + 1))
		{
			uint16_t value = equip.get_stat(es);

			if (value == 0)
				continue;

			// Attack and defence are the equip's own, the rest bonuses.
			bool base = es == EquipStat::Id::WATK || es == EquipStat::Id::MAGIC ||
						es == EquipStat::Id::WDEF || es == EquipStat::Id::MDEF;

			add_line(stat_label(es), (base ? "" : "+") + std::to_string(value), false);
		}

		if (!itemdata.is_cashitem())
			add_line("NUMBER OF UPGRADES AVAILABLE", std::to_string(equip.get_slots()), false);

		int16_t height = STATS_TOP + STAT_STEP * static_cast<int16_t>(stats.size());

		std::string descstr = itemdata.get_desc();

		if (descstr.empty())
		{
			desc = Text();
		} else
		{
			desc = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE, descstr, DESC_WIDTH, true, tooltip::LINE_ADJ);
			height += desc.height();
		}

		size = Point<int16_t>(WIDTH, height + tooltip::BOTTOM);
	}

	void EquipTooltip::draw(Point<int16_t> pos) const
	{
		if (invpos == 0)
			return;

		pos = tooltip::place(pos, size);
		box.draw(pos, size);

		icon.draw_dot(pos + tooltip::DOT_POS);
		name.draw(pos + tooltip::NAME_POS);
		icon.draw(itemicon, pos + tooltip::ICON_POS);

		for (size_t i = 0; i < requirements.size(); i++)
		{
			MapleStat::Id ms = requirements[i];
			bool ok = canequip[ms];
			Point<int16_t> reqpos = pos + REQ_POS + Point<int16_t>(0, REQ_STEP * static_cast<int16_t>(i));

			reqlabels[ms][ok].draw(reqpos);
			reqpos.shift_x(REQ_VALUE_X);

			if (reqvalues[ms].empty())
				reqnone[ok].draw(reqpos);
			else
				reqdigits[ok].draw(reqvalues[ms], DIGIT_STEP, reqpos);
		}

		Point<int16_t> jobpos = pos + JOBS_POS;

		for (size_t i = 0; i < NUM_JOBS; i++)
		{
			const Texture &job = jobs[i][okjobs[i]];
			job.draw(jobpos);
			jobpos.shift_x(job.width() + JOB_GAP);
		}

		line.draw(DrawArgument(pos + LINE_POS));

		Point<int16_t> statpos = pos + Point<int16_t>(0, STATS_TOP);

		for (const StatLine &statline : stats)
		{
			stat_dot.draw(statpos + STAT_DOT);
			statline.label.draw(statpos + Point<int16_t>(STAT_X, 0));
			statline.value.draw(statpos + Point<int16_t>(STAT_X + statline.label.width() + statline.gap, 0));
			statpos.shift_y(STAT_STEP);
		}

		if (!desc.empty())
			desc.draw(statpos + Point<int16_t>(DESC_X, 0));
	}
}
