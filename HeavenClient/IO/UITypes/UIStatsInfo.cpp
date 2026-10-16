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
#include "UIStatsInfo.h"

#include "../UI.h"

#include "../Components/MapleButton.h"

#include "../../Character/ExpTable.h"
#include "../../Gameplay/Stage.h"
#include "../../Net/Packets/PlayerPackets.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// Laid out from footage, against the window's top-left corner.
		constexpr int16_t WIDTH = 175, HEIGHT = 347;
		// The detail panel opens beside the window, level with its stats.
		const Point<int16_t> DETAIL = { 170, 144 };
		constexpr int16_t DETAIL_WIDTH = 177, DETAIL_HEIGHT = 203;

		const Point<int16_t> CLOSE = { 156, 6 };
		const Point<int16_t> DETAIL_BUTTON = { 124, 324 };
		const Point<int16_t> HIDE = { 155, 182 };

		// Values start right of the labels; a row's text has its cap top 4
		// below the row's top, and its AP button sits 3 below it.
		constexpr int16_t VALUE_X = 61;
		constexpr int16_t DETAIL_VALUE_X = 77;
		constexpr int16_t JOB_X = 62;
		constexpr int16_t APUP_X = 153;
		constexpr int16_t LABEL_X = 8;
		// From where a line of text is drawn to its cap top.
		constexpr int16_t TEXT_Y = -9;

		constexpr int16_t ROW_NAME = 32, ROW_JOB = 50, ROW_LEVEL = 78, ROW_GUILD = 96, ROW_HP = 114, ROW_MP = 132,
				ROW_EXP = 150, ROW_FAME = 168, ROW_STR = 244, ROW_DEX = 262, ROW_INT = 280, ROW_LUK = 298;
		// The detail panel's first row, and the step between its rows.
		constexpr int16_t DETAIL_ROW = 7, DETAIL_STEP = 18;
		// The right edge of the AP box's number, and its cap top.
		const Point<int16_t> AP_VALUE = { 84, 218 };
	}

	UIStatsInfo::UIStatsInfo(const CharStats &st) : UIDragElement<PosSTATS>(Point<int16_t>(WIDTH, 20)), stats(st)
	{
		nl::node Stat = NxFiles::ui_classic["UIWindow.img"]["Stat"];

		sprites.emplace_back(Stat["backgrnd"]);
		detail = Stat["backgrnd2"];

		disabled[0] = Stat["Disabled"]["STR"];
		disabled[1] = Stat["Disabled"]["DEX"];
		disabled[2] = Stat["Disabled"]["INT"];
		disabled[3] = Stat["Disabled"]["LUK"];

		nl::node apup = Stat["BtApUp"];

		buttons[Buttons::BT_CLOSE] = std::make_unique<MapleButton>(NxFiles::ui_classic["Basic.img"]["BtClose"], CLOSE);
		buttons[Buttons::BT_HP] = std::make_unique<MapleButton>(apup, Point<int16_t>(APUP_X, ROW_HP + 3));
		buttons[Buttons::BT_MP] = std::make_unique<MapleButton>(apup, Point<int16_t>(APUP_X, ROW_MP + 3));
		buttons[Buttons::BT_STR] = std::make_unique<MapleButton>(apup, Point<int16_t>(APUP_X, ROW_STR + 3));
		buttons[Buttons::BT_DEX] = std::make_unique<MapleButton>(apup, Point<int16_t>(APUP_X, ROW_DEX + 3));
		buttons[Buttons::BT_INT] = std::make_unique<MapleButton>(apup, Point<int16_t>(APUP_X, ROW_INT + 3));
		buttons[Buttons::BT_LUK] = std::make_unique<MapleButton>(apup, Point<int16_t>(APUP_X, ROW_LUK + 3));
		buttons[Buttons::BT_DETAIL] = std::make_unique<MapleButton>(Stat["BtDetail"], DETAIL_BUTTON);
		buttons[Buttons::BT_HIDE] = std::make_unique<MapleButton>(NxFiles::ui_classic["Basic.img"]["BtHide"],
																  DETAIL + HIDE);

		for (auto &label : statlabels)
			label = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK);

		statlabels[StatLabel::AP] = Text(Text::Font::A12M, Text::Alignment::RIGHT, Color::Name::BLACK);
		job_name = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK);

		dimension = Point<int16_t>(WIDTH, HEIGHT);
		set_detail(false);
		update_all_stats();
	}

	void UIStatsInfo::draw(float alpha) const
	{
		if (showdetail)
			detail.draw(position + DETAIL);

		UIElement::draw_sprites(alpha);

		if (is_beginner())
		{
			disabled[2].draw(position + Point<int16_t>(LABEL_X, ROW_INT));
			disabled[3].draw(position + Point<int16_t>(LABEL_X, ROW_LUK));
		}

		if (job_main.is_valid())
		{
			job_main.draw(position + Point<int16_t>(JOB_X, ROW_JOB + 3));
			job_sub.draw(position + Point<int16_t>(JOB_X + 3, ROW_JOB + 15));
		}
		else
		{
			job_name.draw(position + Point<int16_t>(VALUE_X, ROW_JOB + 4 + TEXT_Y));
		}

		const int16_t rows[StatLabel::NUM_MAIN] = {
				ROW_NAME, ROW_LEVEL, ROW_GUILD, ROW_HP, ROW_MP, ROW_EXP, ROW_FAME, 0, ROW_STR, ROW_DEX, ROW_INT, ROW_LUK
		};

		for (size_t i = 0; i < StatLabel::NUM_MAIN; i++)
			if (i != StatLabel::AP)
				statlabels[i].draw(position + Point<int16_t>(VALUE_X, rows[i] + 4 + TEXT_Y));

		statlabels[StatLabel::AP].draw(position + AP_VALUE + Point<int16_t>(0, TEXT_Y));

		if (showdetail)
		{
			for (size_t i = StatLabel::NUM_MAIN; i < StatLabel::NUM_LABELS; i++)
			{
				int16_t row = DETAIL_ROW + DETAIL_STEP * static_cast<int16_t>(i - StatLabel::NUM_MAIN);

				statlabels[i].draw(position + DETAIL + Point<int16_t>(DETAIL_VALUE_X, row + 4 + TEXT_Y));
			}
		}

		UIElement::draw_buttons(alpha);
	}

	void UIStatsInfo::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (pressed && escape)
			deactivate();
	}

	bool UIStatsInfo::is_in_range(Point<int16_t> cursorpos) const
	{
		if (Rectangle<int16_t>(position, position + dimension).contains(cursorpos))
			return true;

		if (!showdetail)
			return false;

		Point<int16_t> detail_pos = position + DETAIL;

		return Rectangle<int16_t>(detail_pos, detail_pos + Point<int16_t>(DETAIL_WIDTH, DETAIL_HEIGHT)).contains(
				cursorpos);
	}

	UIElement::Type UIStatsInfo::get_type() const
	{
		return TYPE;
	}

	void UIStatsInfo::update_all_stats()
	{
		update_ap();
		update_job();

		int16_t level = stats.get_stat(MapleStat::Id::LEVEL);
		int64_t exp = stats.get_exp();
		int64_t percent = level < ExpTable::LEVELCAP ? 100 * exp / ExpTable::values[level] : 0;

		statlabels[StatLabel::NAME].change_text(stats.get_name());
		statlabels[StatLabel::LEVEL].change_text(std::to_string(level));
		statlabels[StatLabel::GUILD].change_text("-");
		statlabels[StatLabel::HP].change_text(std::to_string(stats.get_stat(MapleStat::Id::HP)) + " / " + std::to_string(stats.get_total(EquipStat::Id::HP)));
		statlabels[StatLabel::MP].change_text(std::to_string(stats.get_stat(MapleStat::Id::MP)) + " / " + std::to_string(stats.get_total(EquipStat::Id::MP)));
		statlabels[StatLabel::EXP].change_text(std::to_string(exp) + " (" + std::to_string(percent) + "%)");
		statlabels[StatLabel::FAME].change_text(std::to_string(stats.get_stat(MapleStat::Id::FAME)));
		statlabels[StatLabel::AP].change_text(std::to_string(stats.get_stat(MapleStat::Id::AP)));

		update_basevstotal(StatLabel::STR, MapleStat::Id::STR, EquipStat::Id::STR);
		update_basevstotal(StatLabel::DEX, MapleStat::Id::DEX, EquipStat::Id::DEX);
		update_basevstotal(StatLabel::INT, MapleStat::Id::INT, EquipStat::Id::INT);
		update_basevstotal(StatLabel::LUK, MapleStat::Id::LUK, EquipStat::Id::LUK);

		statlabels[StatLabel::ATTACK].change_text(
				std::to_string(stats.get_mindamage()) + " ~ " + std::to_string(stats.get_maxdamage()));
		statlabels[StatLabel::ATTACK].change_color(stats.is_damage_buffed() ? Color::Name::RED : Color::Name::BLACK);

		// The window adds what INT, DEX and LUK give to the totals; the
		// footage's beginners show these values with no equipment bonuses.
		int32_t dex = stats.get_total(EquipStat::Id::DEX);
		int32_t intl = stats.get_total(EquipStat::Id::INT);
		int32_t luk = stats.get_total(EquipStat::Id::LUK);

		update_buffed(StatLabel::WDEF, EquipStat::Id::WDEF, 0);
		update_buffed(StatLabel::MAGIC, EquipStat::Id::MAGIC, intl);
		update_buffed(StatLabel::MDEF, EquipStat::Id::MDEF, intl);
		update_buffed(StatLabel::ACCURACY, EquipStat::Id::ACC, 0);
		update_buffed(StatLabel::AVOID, EquipStat::Id::AVOID, dex / 4 + luk / 2);
		update_buffed(StatLabel::HANDS, EquipStat::Id::HANDS, dex + intl + luk);

		statlabels[StatLabel::SPEED].change_text(std::to_string(stats.get_total(EquipStat::Id::SPEED)) + "%");
		statlabels[StatLabel::JUMP].change_text(std::to_string(stats.get_total(EquipStat::Id::JUMP)) + "%");
	}

	void UIStatsInfo::update_stat(MapleStat::Id)
	{
		update_all_stats();
	}

	Button::State UIStatsInfo::button_pressed(uint16_t id)
	{
		switch (id)
		{
			case Buttons::BT_CLOSE:
				deactivate();
				break;
			case Buttons::BT_HP:
				send_apup(MapleStat::Id::MAXHP);
				break;
			case Buttons::BT_MP:
				send_apup(MapleStat::Id::MAXMP);
				break;
			case Buttons::BT_STR:
				send_apup(MapleStat::Id::STR);
				break;
			case Buttons::BT_DEX:
				send_apup(MapleStat::Id::DEX);
				break;
			case Buttons::BT_INT:
				send_apup(MapleStat::Id::INT);
				break;
			case Buttons::BT_LUK:
				send_apup(MapleStat::Id::LUK);
				break;
			case Buttons::BT_DETAIL:
				set_detail(true);
				break;
			case Buttons::BT_HIDE:
				set_detail(false);
				break;
			default:
				break;
		}

		return Button::State::NORMAL;
	}

	void UIStatsInfo::send_apup(MapleStat::Id stat) const
	{
		Stage::get().get_player().request(SpendApPacket(stat), ExclRequest::WAIT);
	}

	void UIStatsInfo::set_detail(bool enabled)
	{
		showdetail = enabled;

		buttons[Buttons::BT_DETAIL]->set_active(!enabled);
		buttons[Buttons::BT_HIDE]->set_active(enabled);
	}

	bool UIStatsInfo::is_beginner() const
	{
		return stats.get_job().get_level() == Job::Level::BEGINNER;
	}

	void UIStatsInfo::update_ap()
	{
		bool hasap = stats.get_stat(MapleStat::Id::AP) > 0;
		Button::State state = hasap ? Button::State::NORMAL : Button::State::DISABLED;

		for (int i = Buttons::BT_HP; i <= Buttons::BT_LUK; i++)
			buttons[i]->set_state(state);

		// Beginners can't put AP into HP or MP.
		if (is_beginner())
		{
			buttons[Buttons::BT_HP]->set_state(Button::State::DISABLED);
			buttons[Buttons::BT_MP]->set_state(Button::State::DISABLED);
		}
	}

	void UIStatsInfo::update_job()
	{
		// The job's branch and title are small bitmaps; Cygnus Knights use
		// the premium set, and jobs with neither are written out.
		uint16_t id = stats.get_job().get_id();
		nl::node job = NxFiles::ui_classic["UIWindow.img"]["Stat"]["Job"];
		std::string branch = std::to_string(id / 100 % 10);

		if (id < 1000)
		{
			job_main = job["main"][branch];
			job_sub = job["sub"][branch][std::to_string(id)];
		}
		else if (id < 2000)
		{
			job_main = job["main"]["premium"][branch];
			job_sub = job["sub"]["premium"][branch][std::to_string(id)];
		}
		else
		{
			job_main = {};
			job_sub = {};
		}

		job_name.change_text(stats.get_jobname());
	}

	void UIStatsInfo::update_basevstotal(StatLabel label, MapleStat::Id bstat, EquipStat::Id tstat)
	{
		int32_t base = stats.get_stat(bstat);
		int32_t total = stats.get_total(tstat);
		int32_t delta = total - base;

		std::string stattext = std::to_string(total);

		if (delta)
		{
			stattext += " (" + std::to_string(base);

			if (delta > 0)
				stattext += "+" + std::to_string(delta);
			else if (delta < 0)
				stattext += "-" + std::to_string(-delta);

			stattext += ")";
		}

		statlabels[label].change_text(stattext);
	}

	void UIStatsInfo::update_buffed(StatLabel label, EquipStat::Id stat, int32_t extra)
	{
		int32_t total = stats.get_total(stat) + extra;
		int32_t delta = stats.get_buffdelta(stat);

		std::string stattext = std::to_string(total);
		Color::Name color = Color::Name::BLACK;

		if (delta)
		{
			stattext += " (" + std::to_string(total - delta);

			if (delta > 0)
			{
				stattext += "+" + std::to_string(delta);
				color = Color::Name::RED;
			} else
			{
				stattext += "-" + std::to_string(-delta);
				color = Color::Name::BLUE;
			}

			stattext += ")";
		}

		statlabels[label].change_text(stattext);
		statlabels[label].change_color(color);
	}
}
