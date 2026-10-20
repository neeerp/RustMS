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
#include "UISkillBook.h"
#include "UISkillMacro.h"

#include "../UI.h"

#include "../Components/AreaButton.h"
#include "../Components/MapleButton.h"
#include "../Components/SkillRecommendation.h"

#include "../../Character/SkillId.h"
#include "../../Data/JobData.h"
#include "../../Data/SkillData.h"
#include "../../Gameplay/Stage.h"
#include "../../Util/NxFiles.h"

#include "../../Net/Packets/PlayerPackets.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// The window, measured from period footage.
		constexpr Point<int16_t> CLOSE = Point<int16_t>(156, 6);
		constexpr Point<int16_t> TAB_STRIP = Point<int16_t>(3, 23);
		constexpr int16_t TAB_FILL = 22;
		// The book icon's origin and the centre of the job name above it.
		constexpr Point<int16_t> BOOK = Point<int16_t>(6, 87);
		constexpr Point<int16_t> BOOK_NAME = Point<int16_t>(103, 62);
		// The first row's top-left; each row's parts are offsets from it.
		constexpr Point<int16_t> ROW = Point<int16_t>(7, 99);
		constexpr int16_t ROW_WIDTH = 141;
		constexpr int16_t ROW_SHADE = 35;
		constexpr Point<int16_t> ROW_LINE = Point<int16_t>(0, 37);
		constexpr Point<int16_t> ROW_ICON = Point<int16_t>(2, 2);
		constexpr Point<int16_t> ROW_NAME = Point<int16_t>(40, -5);
		constexpr Point<int16_t> ROW_LEVEL = Point<int16_t>(40, 13);
		constexpr Point<int16_t> ROW_SPUP = Point<int16_t>(124, 20);
		// The recommended skill's frame, ignoring its origin like v83.
		constexpr Point<int16_t> ROW_RECOMMEND = Point<int16_t>(36, 0);
		constexpr int16_t SLIDER_X = 153;
		constexpr Point<int16_t> SP_LABEL = Point<int16_t>(110, 259);
		// v83 FUN_008aaa88.
		constexpr Point<int16_t> MACRO = Point<int16_t>(120, 265);
		constexpr int16_t ICON_SIZE = 32;
	}

	UISkillBook::SkillIcon::SkillIcon(int32_t id) : skill_id(id) {}

	void UISkillBook::SkillIcon::drop_on_bindings(Point<int16_t> cursorposition, bool remove) const
	{
		auto keyconfig = UI::get().get_element<UIKeyConfig>();
		Keyboard::Mapping mapping = get_mapping();

		if (remove)
			keyconfig->unstage_mapping(mapping);
		else
			keyconfig->stage_mapping(cursorposition, mapping);
	}

	Keyboard::Mapping UISkillBook::SkillIcon::get_mapping() const
	{
		return Keyboard::Mapping(KeyType::SKILL, skill_id);
	}

	Icon::IconType UISkillBook::SkillIcon::get_type()
	{
		return Icon::IconType::SKILL;
	}

	UISkillBook::SkillDisplayMeta::SkillDisplayMeta(int32_t i, int32_t l) : id(i), level(l)
	{
		const SkillData &data = SkillData::get(id);

		Texture ntx = data.get_icon(SkillData::Icon::NORMAL);
		Texture dtx = data.get_icon(SkillData::Icon::DISABLED);
		Texture motx = data.get_icon(SkillData::Icon::MOUSEOVER);
		icon = std::make_unique<StatefulIcon>(std::make_unique<SkillIcon>(id), ntx, dtx, motx);

		std::string namestr = data.get_name();
		std::string levelstr = std::to_string(level);

		name_text = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, namestr);
		level_text = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, levelstr);

		constexpr uint16_t MAX_NAME_WIDTH = ROW_WIDTH - ROW_NAME.x() - 2;
		size_t overhang = 3;

		while (name_text.width() > MAX_NAME_WIDTH && overhang < namestr.size())
		{
			namestr.replace(namestr.end() - overhang, namestr.end(), "..");
			overhang += 1;

			name_text.change_text(namestr);
		}
	}

	void UISkillBook::SkillDisplayMeta::draw(Point<int16_t> position) const
	{
		icon->draw(position + ROW_ICON);
		name_text.draw(position + ROW_NAME);
		level_text.draw(position + ROW_LEVEL);
	}

	int32_t UISkillBook::SkillDisplayMeta::get_id() const
	{
		return id;
	}

	int32_t UISkillBook::SkillDisplayMeta::get_level() const
	{
		return level;
	}

	StatefulIcon *UISkillBook::SkillDisplayMeta::get_icon() const
	{
		return icon.get();
	}

	UISkillBook::UISkillBook(const CharStats &in_stats, const SkillBook &in_skillbook)
			: UIDragElement<PosSKILL>(), stats(in_stats), skillbook(in_skillbook), sp(0), beginner_sp(0), tab(0),
			  offset(0), grabbing(false)
	{
		nl::node Skill = NxFiles::ui_classic["UIWindow.img"]["Skill"];
		nl::node backgrnd = Skill["backgrnd"];

		sprites.emplace_back(backgrnd);

		skilld = Skill["skill0"];
		skille = Skill["skill1"];
		line = Skill["line"];
		recommend = Skill["recommend"]["0"];

		tabs = ClassicTabs(ClassicTabs::TAB5, Skill["Tab"]["enabled"], Skill["Tab"]["disabled"], TAB_FILL);

		buttons[Buttons::BT_CLOSE] = std::make_unique<MapleButton>(NxFiles::ui_classic["Basic.img"]["BtClose"], CLOSE);

		for (uint16_t i = 0; i < TABS; i++)
		{
			Point<int16_t> tab_pos = TAB_STRIP + Point<int16_t>(tabs.pitch() * i, 0);

			buttons[Buttons::BT_TAB0 + i] = std::make_unique<AreaButton>(tab_pos,
																		 Point<int16_t>(tabs.pitch(), tabs.height()));
			buttons[Buttons::BT_TAB0 + i]->set_tab();
		}

		for (uint16_t i = 0; i < ROWS; i++)
		{
			Point<int16_t> spup_pos = ROW + ROW_SPUP + Point<int16_t>(0, ROW_HEIGHT * i);

			buttons[Buttons::BT_SPUP0 + i] = std::make_unique<MapleButton>(Skill["BtSpUp"], spup_pos);
		}

		buttons[Buttons::BT_MACRO] = std::make_unique<MapleButton>(NxFiles::ui_classic["Basic.img"]["BtMacro"], MACRO);

		booktext = Text(Text::Font::A10M, Text::Alignment::CENTER, Color::Name::WHITE);
		splabel = Text(Text::Font::A12M, Text::Alignment::RIGHT, Color::Name::BLACK);

		Range<int16_t> rows = Range<int16_t>(ROW.y(), ROW.y() + ROW_HEIGHT * (ROWS - 1) + 21);

		slider = Slider(
				Slider::Type::CLASSIC, rows, SLIDER_X, ROWS, 1,
				[&](bool upwards)
				{
					int16_t shift = upwards ? -1 : 1;
					bool above = offset + shift >= 0;
					bool below = offset + ROWS + shift <= static_cast<int16_t>(skills.size());

					if (above && below)
						change_offset(offset + shift);
				}
		);

		change_job(stats.get_stat(MapleStat::Id::JOB));

		dimension = Texture(backgrnd).get_dimensions();
		dragarea = Point<int16_t>(dimension.x(), 20);
	}

	void UISkillBook::draw(float alpha) const
	{
		UIElement::draw_sprites(alpha);

		tabs.draw(position + TAB_STRIP, tab_count(), tab);

		bookicon.draw(position + BOOK);
		booktext.draw(position + BOOK_NAME);
		splabel.draw(position + SP_LABEL);

		for (int16_t i = 0; i < ROWS; i++)
		{
			size_t index = offset + i;

			if (index >= skills.size())
				break;

			Point<int16_t> row_pos = position + ROW + Point<int16_t>(0, ROW_HEIGHT * i);

			if (check_required(skills[index].get_id()))
			{
				skille.draw(row_pos);
			} else
			{
				skilld.draw(row_pos);
				skills[index].get_icon()->set_state(StatefulIcon::State::DISABLED);
			}

			if (skills[index].get_id() == recommended)
				recommend.draw(row_pos + ROW_RECOMMEND + recommend.get_origin());

			skills[index].draw(row_pos);

			if (i < ROWS - 1 && index + 1 < skills.size())
				line.draw(row_pos + ROW_LINE);
		}

		slider.draw(position);

		UIElement::draw_buttons(alpha);
	}

	Button::State UISkillBook::button_pressed(uint16_t id)
	{
		switch (id)
		{
			case Buttons::BT_CLOSE:
				close();
				break;
			case Buttons::BT_TAB0:
			case Buttons::BT_TAB1:
			case Buttons::BT_TAB2:
			case Buttons::BT_TAB3:
			case Buttons::BT_TAB4:
				clear_tooltip();
				change_tab(id - Buttons::BT_TAB0);

				return Button::State::PRESSED;
			case Buttons::BT_SPUP0:
			case Buttons::BT_SPUP1:
			case Buttons::BT_SPUP2:
			case Buttons::BT_SPUP3:
			{
				size_t index = offset + id - Buttons::BT_SPUP0;

				if (index < skills.size())
					spend_sp(skills[index].get_id());
			}
				break;
			case Buttons::BT_MACRO:
				clear_tooltip();

				if (auto macros = UI::get().emplace<UISkillMacro>(position))
					macros->dock(position);

				break;
			default:
				break;
		}

		return Button::State::NORMAL;
	}

	void UISkillBook::toggle_active()
	{
		if (active)
			close_macros();

		UIElement::toggle_active();

		clear_tooltip();
	}

	void UISkillBook::update()
	{
		UIElement::update();

		// The macro window keeps to the skill window's right.
		if (auto macros = UI::get().get_element<UISkillMacro>())
			macros->dock(position);
	}

	void UISkillBook::doubleclick(Point<int16_t> cursorpos)
	{
		int16_t row = row_by_position(cursorpos - position);

		if (row < 0)
			return;

		int32_t skill_id = skills[offset + row].get_id();
		int32_t skill_level = skillbook.get_level(skill_id);

		if (skill_level > 0)
			Stage::get().get_combat().use_move(skill_id);
	}

	void UISkillBook::remove_cursor()
	{
		UIDragElement::remove_cursor();

		slider.remove_cursor();
	}

	Cursor::State UISkillBook::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		Cursor::State dstate = UIDragElement::send_cursor(clicked, cursorpos);

		if (dragged)
			return dstate;

		Point<int16_t> cursor_relative = cursorpos - position;

		if (slider.isenabled())
		{
			if (Cursor::State new_state = slider.send_cursor(cursor_relative, clicked))
			{
				clear_tooltip();

				return new_state;
			}
		}

		if (!grabbing)
		{
			for (auto &skill : skills)
				skill.get_icon()->set_state(StatefulIcon::State::NORMAL);

			int16_t row = row_by_position(cursor_relative);
			Point<int16_t> icon_pos = ROW + ROW_ICON + Point<int16_t>(0, ROW_HEIGHT * row);
			Rectangle<int16_t> icon_bounds = Rectangle<int16_t>(icon_pos, icon_pos + Point<int16_t>(ICON_SIZE, ICON_SIZE));

			if (row >= 0 && icon_bounds.contains(cursor_relative))
			{
				const SkillDisplayMeta &skill = skills[offset + row];
				int32_t skill_id = skill.get_id();

				if (clicked)
				{
					clear_tooltip();
					grabbing = true;

					int32_t skill_level = skillbook.get_level(skill_id);

					if (skill_level > 0 && !SkillData::get(skill_id).is_passive())
					{
						skill.get_icon()->start_drag(cursor_relative - icon_pos);
						UI::get().drag_icon(skill.get_icon());

						return Cursor::State::GRABBING;
					}

					return Cursor::State::IDLE;
				}

				skill.get_icon()->set_state(StatefulIcon::State::MOUSEOVER);
				show_skill(skill_id);

				return Cursor::State::IDLE;
			}

			clear_tooltip();
		} else
		{
			grabbing = false;
		}

		return UIElement::send_cursor(clicked, cursorpos);
	}

	void UISkillBook::send_scroll(double yoffset)
	{
		if (slider.isenabled())
			slider.send_scroll(yoffset);
	}

	void UISkillBook::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (pressed)
		{
			if (escape)
			{
				close();
			} else if (keycode == KeyAction::Id::TAB)
			{
				clear_tooltip();
				change_tab((tab + 1) % tab_count());
			}
		}
	}

	UIElement::Type UISkillBook::get_type() const
	{
		return TYPE;
	}

	void UISkillBook::update_stat(MapleStat::Id stat, int16_t value)
	{
		switch (stat)
		{
			case MapleStat::Id::JOB:
				change_job(value);
				break;
			case MapleStat::Id::SP:
				change_sp();
				break;
		}
	}

	void UISkillBook::update_skills(int32_t)
	{
		// Keep the list where it was, so raising a skill doesn't scroll away.
		uint16_t old_offset = offset;

		load_skills();

		if (old_offset + ROWS > skills.size())
			old_offset = 0;

		slider.setrows(old_offset, ROWS, skills.size());
		change_offset(old_offset);
		change_sp();
	}

	void UISkillBook::change_job(uint16_t id)
	{
		job.change_job(id);

		for (uint16_t i = 0; i < TABS; i++)
			buttons[Buttons::BT_TAB0 + i]->set_active(i < tab_count());

		change_tab(tab_count() - 1);
	}

	void UISkillBook::change_sp()
	{
		uint16_t level = stats.get_stat(MapleStat::Id::LEVEL);

		// Beginner skills take the points of the first six levels, which the
		// server doesn't count in SP.
		beginner_sp = std::min<int16_t>(level - 1, 6);

		for (int32_t skill_id : { SkillId::Id::THREE_SNAILS, SkillId::Id::HEAL, SkillId::Id::FEATHER })
			beginner_sp -= skillbook.get_level(skill_id);

		sp = stats.get_stat(MapleStat::Id::SP);

		if (joblevel_by_tab(tab) == Job::Level::BEGINNER)
			splabel.change_text(std::to_string(beginner_sp));
		else
			splabel.change_text(std::to_string(sp));

		change_offset(offset);
	}

	void UISkillBook::change_tab(uint16_t new_tab)
	{
		buttons[Buttons::BT_TAB0 + tab]->set_state(Button::NORMAL);
		buttons[Buttons::BT_TAB0 + new_tab]->set_state(Button::PRESSED);
		tab = new_tab;

		load_skills();

		slider.setrows(ROWS, skills.size());
		change_offset(0);
		change_sp();
	}

	void UISkillBook::load_skills()
	{
		skills.clear();

		uint16_t subid = job.get_subjob(joblevel_by_tab(tab));
		const JobData &data = JobData::get(subid);

		bookicon = data.get_icon();
		booktext.change_text(data.get_name());

		for (int32_t skill_id : data.get_skills())
		{
			int32_t level = skillbook.get_level(skill_id);
			int32_t masterlevel = skillbook.get_masterlevel(skill_id);

			bool invisible = SkillData::get(skill_id).is_invisible();

			if (invisible && masterlevel == 0)
				continue;

			skills.emplace_back(skill_id, level);
		}

		// FUN_008abf71
		int32_t spent = 0;

		for (int32_t skill_id : data.get_skills())
			spent += skillbook.get_level(skill_id);

		std::vector<std::pair<int32_t, int32_t>> table;

		for (nl::node entry : nl::nx::etc["RecommendSkill.img"][std::to_string(subid)])
			table.emplace_back(std::stoi(entry.name()), entry.get_integer());

		recommended = recommended_skill(table, spent);
	}

	void UISkillBook::change_offset(uint16_t new_offset)
	{
		offset = new_offset;

		for (int16_t i = 0; i < ROWS; i++)
		{
			auto &button = buttons[Buttons::BT_SPUP0 + i];
			size_t index = offset + i;

			button->set_active(index < skills.size());

			if (index < skills.size())
				button->set_state(can_raise(skills[index].get_id()) ? Button::State::NORMAL : Button::State::DISABLED);
		}
	}

	void UISkillBook::show_skill(int32_t id)
	{
		int32_t skill_id = id;
		int32_t level = skillbook.get_level(id);
		int32_t masterlevel = skillbook.get_masterlevel(id);
		int64_t expiration = skillbook.get_expiration(id);

		UI::get().show_skill(Tooltip::Parent::SKILLBOOK, skill_id, level, masterlevel, expiration);
	}

	void UISkillBook::clear_tooltip()
	{
		UI::get().clear_tooltip(Tooltip::Parent::SKILLBOOK);
	}

	bool UISkillBook::can_raise(int32_t skill_id) const
	{
		if (joblevel_by_tab(tab) == Job::Level::BEGINNER)
		{
			if (beginner_sp <= 0)
				return false;
		} else if (sp <= 0)
		{
			return false;
		}

		int32_t level = skillbook.get_level(skill_id);
		int32_t masterlevel = skillbook.get_masterlevel(skill_id);

		if (masterlevel == 0)
			masterlevel = SkillData::get(skill_id).get_masterlevel();

		if (level >= masterlevel)
			return false;

		switch (skill_id)
		{
			case SkillId::Id::ANGEL_BLESSING:
				return false;
			default:
				return check_required(skill_id);
		}
	}

	void UISkillBook::spend_sp(int32_t skill_id)
	{
		Stage::get().get_player().request(SpendSpPacket(skill_id), ExclRequest::WAIT);
	}

	Job::Level UISkillBook::joblevel_by_tab(uint16_t t) const
	{
		switch (t)
		{
			case 1:
				return Job::Level::FIRST;
			case 2:
				return Job::Level::SECOND;
			case 3:
				return Job::Level::THIRD;
			case 4:
				return Job::Level::FOURTH;
			default:
				return Job::Level::BEGINNER;
		}
	}

	uint16_t UISkillBook::tab_count() const
	{
		return job.get_level() - Job::Level::BEGINNER + 1;
	}

	int16_t UISkillBook::row_by_position(Point<int16_t> cursorpos) const
	{
		Point<int16_t> relative = cursorpos - ROW;

		if (relative.x() < 0 || relative.x() >= ROW_WIDTH || relative.y() < 0)
			return -1;

		int16_t row = relative.y() / ROW_HEIGHT;

		if (row >= ROWS || relative.y() % ROW_HEIGHT >= ROW_SHADE || offset + row >= skills.size())
			return -1;

		return row;
	}

	void UISkillBook::close()
	{
		close_macros();
		clear_tooltip();
		deactivate();
	}

	void UISkillBook::close_macros()
	{
		// v83 destroys the macro window with the skill window.
		if (auto macros = UI::get().get_element<UISkillMacro>())
			if (macros->is_active())
				macros->close();
	}

	bool UISkillBook::check_required(int32_t id) const
	{
		std::unordered_map<int32_t, int32_t> required = skillbook.collect_required(id);

		if (required.size() <= 0)
			required = SkillData::get(id).get_reqskills();

		for (auto reqskill : required)
		{
			int32_t reqskill_level = skillbook.get_level(reqskill.first);
			int32_t req_level = reqskill.second;

			if (reqskill_level < req_level)
				return false;
		}

		return true;
	}
}
