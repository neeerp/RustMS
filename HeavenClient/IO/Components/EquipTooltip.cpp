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

#include "../../Character/Inventory/CashUpgrades.h"
#include "../../Character/Inventory/Equip.h"
#include "../../Character/Inventory/ItemExpiry.h"
#include "../../Character/Inventory/ItemProperties.h"
#include "../../Character/Marriage.h"
#include "../../Data/WeaponData.h"
#include "../../Gameplay/Stage.h"
#include "../../Util/NxFiles.h"

#include <algorithm>

namespace ms
{
	namespace
	{
		// Measured from period footage. A stat line wider than the box widens it to 20 past the line (0x8f3b4f).
		constexpr int16_t WIDTH = 236;
		constexpr int16_t LINE_MARGIN = 20;
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
		// The expiry line under the name, after a dot like the stat lines'. Everything under it moves down
		// (FUN_008e8252).
		const Point<int16_t> EXPIRY_DOT = { 10, 33 };
		const Point<int16_t> EXPIRY_POS = { 16, 22 };
		constexpr int16_t EXPIRY_HEIGHT = 16;
		// The orange lines under the name, centred and 19 apart, as on other items. A first line wider than the box
		// wraps from x 25 instead, 2 lower, and each line takes 19 for every box width of it (FUN_008e8252).
		constexpr int16_t PROPERTY_Y = 25;
		constexpr int16_t PROPERTY_STEP = 19;
		const Point<int16_t> PROPERTY_WRAP_POS = { 25, 27 };
		constexpr int16_t PROPERTY_WRAP_WIDTH = 178;
		// An owner's line takes the top, centred, and moves everything else down (FUN_008f49bc, report c011).
		constexpr int16_t OWNER_HEIGHT = 19;

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
		width(WIDTH), line(WIDTH - 2 * LINE_POS.x(), 1, Color::Name::WHITE, 0.9f)
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
		wrapped = false;
		properties_height = 0;
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
			case Tooltip::Parent::STORAGE:
			case Tooltip::Parent::ITEMMEGAPHONE:
			case Tooltip::Parent::VICIOUSHAMMER:
			case Tooltip::Parent::VEGASPELL:
			case Tooltip::Parent::ITEMPROTECTOR:
			case Tooltip::Parent::KARMASCISSORS:
			case Tooltip::Parent::VEGARESULT:
			case Tooltip::Parent::ENCHANTSKILL:
			case Tooltip::Parent::DUEY:
				invtype = InventoryType::Id::EQUIP;
				break;
			case Tooltip::Parent::EQUIPINVENTORY:
				invtype = InventoryType::Id::EQUIPPED;
				break;
			default:
				invtype = InventoryType::Id::NONE;
		}

		const Inventory &inventory = Stage::get().get_player().get_inventory();
		auto oequip = inventory.get_equip(invtype, invpos);

		if (!oequip)
		{
			invpos = 0;
			return;
		}

		const Equip &equip = *oequip;
		// Equips bought in the Cash Shop keep its serial.
		int64_t cash_id = inventory.get_cash_id(invtype, invpos);
		build(equip, item_expiry::line(equip.get_expiration(), cash_id != 0, equip.get_flags() & item_expiry::SEALED), "",
			  cash_id);
	}

	namespace
	{
		// An equip as Character.nx makes it, before any upgrade (CItemInfo FUN_005d5d95).
		Equip template_equip(int32_t item_id)
		{
			const EquipData &equipdata = EquipData::get(item_id);
			EnumMap<EquipStat::Id, uint16_t> stats;

			for (EquipStat::Id es : EquipStat::values)
				stats[es] = static_cast<uint16_t>(equipdata.get_defstat(es));

			return Equip(item_id, 0, "", 0, equipdata.get_slots(), 0, stats, 0, 0, 0);
		}
	}

	void EquipTooltip::set_commodity(int32_t item_id, int32_t period)
	{
		parent = Tooltip::Parent::CASHSHOP;
		invpos = -1;
		build(template_equip(item_id), item_expiry::commodity_line(item_id, period), "");
	}

	void EquipTooltip::set_cash_item(int32_t item_id, int64_t expiration, const std::string &gift_from)
	{
		parent = Tooltip::Parent::CASHSHOP;
		invpos = -1;
		build(template_equip(item_id), item_expiry::line(expiration, true, false),
			gift_from.empty() ? "" : "IT'S A GIFT FROM '" + gift_from + "'");
	}

	void EquipTooltip::build(const Equip &equip, const std::string &expirystr, const std::string &giftstr,
							 int64_t cash_id)
	{
		const CharStats &charstats = Stage::get().get_player().get_stats();
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

		const std::string &ownerstr = equip.get_owner();
		owner = ownerstr.empty() ? Text() : Text(Text::Font::A12B, Text::Alignment::LEFT, Color::Name::TIPORANGE, ownerstr);
		owner_s = ownerstr.empty() ? Text() : Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE, "'s");

		expiry = expirystr.empty() ? Text() : Text(Text::Font::A11M, Text::Alignment::LEFT, Color::Name::WHITE, expirystr);
		gift = giftstr.empty() ? Text() : Text(Text::Font::A11M, Text::Alignment::LEFT, Color::Name::WHITE, giftstr);

		const item_properties::Info &info = itemdata.get_properties();
		item_properties::Lines lines = item_properties::equip(info, equip.get_flags(), !expirystr.empty());
		properties = {};
		wrapped = false;
		properties_height = 0;

		for (size_t i = 0; i < properties.size(); i++)
		{
			const std::string &line = i == 0 ? lines.first : lines.second;

			if (line.empty())
				continue;

			properties[i] = Text(Text::Font::A12M, Text::Alignment::CENTER, Color::Name::TIPORANGE, line);
			int16_t line_width = properties[i].width();
			properties_height += PROPERTY_STEP * (line_width / WIDTH + 1);

			if (i == 0 && line_width > WIDTH)
			{
				properties[i] = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::TIPORANGE, line,
									 PROPERTY_WRAP_WIDTH, true, tooltip::LINE_ADJ);
				wrapped = true;
			}
		}

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
			statline.label = Text(Text::Font::A11M, Text::Alignment::LEFT, Color::Name::WHITE,
								  label.empty() ? "" : label + " :");
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

		// Then a bow's knock-back rate, the lines of spikes and cold
		// protection, and the slots left of an equip that came with some
		// (FUN_008eca0c).
		if (int32_t knockback = equipdata.get_knockback(); knockback > 0)
			add_line("THE RATE OF KNOCK-BACK", std::to_string(knockback) + "%", false);

		if (equip.get_flags() & item_properties::EQUIP_SPIKES)
			add_line("", "ADD PREVENT SLIPPING", true);

		if (equip.get_flags() & item_properties::EQUIP_COLD)
			add_line("", "ADD PREVENT COLDNESS", true);

		if (equipdata.get_slots() > 0)
			add_line("NUMBER OF UPGRADES AVAILABLE", std::to_string(equip.get_slots()), false);

		// And the Vicious' Hammers it has had, if it could take one (0x8e9355), in one string without a bold value.
		if (vicious_hammer::takes(item_id, equipdata.get_slots()))
			add_line("", "NUMBER OF VICIOUS' HAMMER APPLIED : " + vicious_hammer::tooltip_line(equip.get_vicious()), true);

		width = WIDTH;

		for (const StatLine &statline : stats)
			width = std::max<int16_t>(width, statline.label.width() + statline.gap + statline.value.width() + LINE_MARGIN);

		line.setwidth(width - 2 * LINE_POS.x());

		int16_t height = properties_height + STATS_TOP + STAT_STEP * static_cast<int16_t>(stats.size());

		if (!expiry.empty())
			height += EXPIRY_HEIGHT;

		if (!gift.empty())
			height += EXPIRY_HEIGHT;

		if (!owner.empty())
			height += OWNER_HEIGHT;

		// A wedding ring names the partner in place of its description (0x8e9109), and so, we take it, does
		// a crush or friendship ring of the player's pairs (0x8e7975).
		const Player &player = Stage::get().get_player();
		std::string descstr = marriage::tooltip_line(item_id, player.get_marriage(), charstats.get_female());

		if (descstr.empty())
			descstr = marriage::pair_tooltip_line(item_id, cash_id, player.get_couple_rings());

		if (descstr.empty())
			descstr = itemdata.get_desc();

		// One that could trade with Scissors of Karma says so.

		if (info.trade_available && !(equip.get_flags() & item_properties::EQUIP_KARMA))
			descstr = item_properties::with_karma_hint(descstr);

		if (descstr.empty())
		{
			desc = Text();
		} else
		{
			desc = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE, descstr, DESC_WIDTH, true, tooltip::LINE_ADJ);
			height += desc.height();
		}

		size = Point<int16_t>(width, height + tooltip::BOTTOM);
	}

	void EquipTooltip::draw(Point<int16_t> pos) const
	{
		if (invpos == 0)
			return;

		pos = tooltip::place(pos, size);
		box.draw(pos, size);

		if (!owner.empty())
		{
			int16_t left = width / 2 - (owner.width() + owner_s.width()) / 2;
			owner.draw(pos + Point<int16_t>(left, tooltip::NAME_POS.y()));
			owner_s.draw(pos + Point<int16_t>(left + owner.width(), tooltip::NAME_POS.y()));
			pos.shift_y(OWNER_HEIGHT);
		}

		icon.draw_dot(pos + tooltip::DOT_POS);
		name.draw(pos + tooltip::NAME_POS);

		if (wrapped)
			properties[0].draw(pos + PROPERTY_WRAP_POS);
		else if (!properties[0].empty())
			properties[0].draw(pos + Point<int16_t>(width / 2, PROPERTY_Y));

		if (!properties[1].empty())
			properties[1].draw(pos + Point<int16_t>(width / 2, PROPERTY_Y + PROPERTY_STEP));

		pos.shift_y(properties_height);

		// Each dot line, the date and then the sender, moves the rest down (FUN_008e8252).
		for (const Text *line : { &expiry, &gift })
		{
			if (line->empty())
				continue;

			stat_dot.draw(pos + EXPIRY_DOT);
			line->draw(pos + EXPIRY_POS);
			pos.shift_y(EXPIRY_HEIGHT);
		}

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
