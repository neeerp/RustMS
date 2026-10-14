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
#include "UIUserList.h"

#include "UIChatBar.h"
#include "UIEnterText.h"

#include "../UI.h"

#include "../Components/AreaButton.h"
#include "../Components/MapleButton.h"

#include "../../Character/Job.h"
#include "../../Gameplay/Stage.h"
#include "../../Util/NxFiles.h"

#include "../../Net/Packets/GameplayPackets.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// The window, measured from period footage.
		constexpr Point<int16_t> CLOSE = Point<int16_t>(291, 7);
		constexpr Point<int16_t> TAB_STRIP = Point<int16_t>(11, 23);
		constexpr int16_t TAB_FILL = 46;
		// The party list's top-left. Each section is a title bar, column
		// headings, one row per member (or "none"), and a bottom edge.
		constexpr Point<int16_t> LIST = Point<int16_t>(15, 51);
		constexpr int16_t HEADER = 27;
		constexpr int16_t COLUMNS = 18;
		constexpr int16_t ROW = 18;
		constexpr int16_t FOOTER = 4;
		constexpr int16_t SECTION_GAP = 10;
		constexpr Point<int16_t> HIGHLIGHT_SIZE = Point<int16_t>(278, ROW);
		// Offsets in a row, from the list's left edge and the row's top.
		constexpr int16_t STAR_X = 4;
		constexpr int16_t NAME_X = 26;
		constexpr int16_t JOB_X = 130;
		constexpr int16_t LEVEL_X = 224;
		constexpr int16_t TEXT_Y = -3;
		constexpr Point<int16_t> COUNT = Point<int16_t>(270, 2);
		// The two rows of party buttons.
		constexpr Point<int16_t> BUTTONS = Point<int16_t>(12, 341);
		constexpr Point<int16_t> BUTTON_PITCH = Point<int16_t>(60, 22);
		constexpr size_t PARTY_CAPACITY = 6;
		constexpr size_t NAME_LIMIT = 12;
	}

	bool UIUserList::Shown::operator==(const Shown& other) const
	{
		return id == other.id && name == other.name && job == other.job && level == other.level &&
			   channel == other.channel && map_id == other.map_id;
	}

	UIUserList::UIUserList(uint16_t t)
			: UIDragElement<PosUSERLIST>(), tab(t), party(Stage::get().get_player().get_party())
	{
		nl::node UserList = NxFiles::ui_classic["UIWindow.img"]["UserList"];
		nl::node Party = UserList["Party"];
		nl::node backgrnd = UserList["backgrnd"];

		sprites.emplace_back(backgrnd);

		tabs = ClassicTabs(UserList["Tab"]["enabled"], UserList["Tab"]["disabled"], TAB_FILL);

		buttons[Buttons::BT_CLOSE] = std::make_unique<MapleButton>(NxFiles::ui_classic["Basic.img"]["BtClose"], CLOSE);

		for (uint16_t i = 0; i < Tab::NUM_TABS; i++)
		{
			Point<int16_t> tab_pos = TAB_STRIP + Point<int16_t>(tabs.pitch() * i, 0);

			buttons[Buttons::BT_TAB0 + i] = std::make_unique<AreaButton>(tab_pos,
																		 Point<int16_t>(tabs.pitch(), ClassicTabs::HEIGHT));
		}

		const char* party_buttons[] = {
				"BtHP", "BtChangeBoss", "BtWhisper", "BtChat", "BtSearch",
				"BtCreate", "BtWithdraw", "BtKick", "BtInvite"
		};

		for (uint16_t i = 0; i < std::size(party_buttons); i++)
		{
			Point<int16_t> slot = Point<int16_t>(i % 5, i / 5);
			Point<int16_t> button_pos = BUTTONS + Point<int16_t>(BUTTON_PITCH.x() * slot.x(), BUTTON_PITCH.y() * slot.y());

			buttons[Buttons::BT_HP + i] = std::make_unique<MapleButton>(Party[party_buttons[i]], button_pos);
		}

		online.header = Party["party0"];
		offline.header = Party["party4"];
		columns = Party["party5"];
		row = Party["party1"];
		bottom = Party["party2"];
		highlight = Party["party3"];
		star = Party["icon0"];

		online.count = Text(Text::Font::A11M, Text::Alignment::RIGHT, Color::Name::WHITE);
		offline.count = Text(Text::Font::A11M, Text::Alignment::RIGHT, Color::Name::WHITE);
		none = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, "none");

		dimension = Texture(backgrnd).get_dimensions();
		dragarea = Point<int16_t>(dimension.x(), 20);

		rebuild();
		change_tab(tab);
	}

	void UIUserList::draw(float alpha) const
	{
		UIElement::draw_sprites(alpha);

		tabs.draw(position + TAB_STRIP, Tab::NUM_TABS, tab);

		if (tab == Tab::PARTY)
		{
			draw_section(online);
			draw_section(offline);
		}

		UIElement::draw_buttons(alpha);
	}

	void UIUserList::draw_section(const Section& section) const
	{
		Point<int16_t> top = position + Point<int16_t>(LIST.x(), section.top);

		section.header.draw(top);
		section.count.draw(top + COUNT);
		columns.draw(top + Point<int16_t>(1, HEADER));

		int16_t first = section.top + HEADER + COLUMNS;

		if (section.lines.empty())
		{
			Point<int16_t> row_pos = position + Point<int16_t>(LIST.x(), first);

			row.draw(row_pos + Point<int16_t>(1, 0));
			none.draw(row_pos + Point<int16_t>(NAME_X, TEXT_Y));
		}

		for (const Line& line : section.lines)
		{
			Point<int16_t> row_pos = position + Point<int16_t>(LIST.x(), line.y);

			// The row art is clear inside its border, so the highlight shows
			// through it.
			if (line.id == selected)
				highlight.draw(DrawArgument(row_pos + Point<int16_t>(1, 0), HIGHLIGHT_SIZE));

			row.draw(row_pos + Point<int16_t>(1, 0));

			if (line.leader)
				star.draw(row_pos + Point<int16_t>(STAR_X, 2));

			line.name.draw(row_pos + Point<int16_t>(NAME_X, TEXT_Y));
			line.job.draw(row_pos + Point<int16_t>(JOB_X, TEXT_Y));
			line.level.draw(row_pos + Point<int16_t>(LEVEL_X, TEXT_Y));
		}

		bottom.draw(position + Point<int16_t>(LIST.x() + 1, section.bottom - FOOTER));
	}

	void UIUserList::update()
	{
		UIElement::update();

		int32_t mapid = Stage::get().get_mapid();
		std::vector<Shown> now = currently_shown();

		if (now != shown || party.get_leader() != shown_leader || mapid != shown_mapid)
			rebuild();

		update_buttons();
	}

	Cursor::State UIUserList::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		Cursor::State dstate = UIDragElement::send_cursor(clicked, cursorpos);

		if (dragged)
			return dstate;

		Point<int16_t> cursor_relative = cursorpos - position;

		if (clicked && tab == Tab::PARTY)
		{
			for (const Section* section : {&online, &offline})
			{
				for (const Line& line : section->lines)
				{
					Point<int16_t> row_pos = Point<int16_t>(LIST.x() + 1, line.y);
					Rectangle<int16_t> bounds = Rectangle<int16_t>(row_pos, row_pos + HIGHLIGHT_SIZE);

					if (bounds.contains(cursor_relative))
					{
						selected = line.id;
						update_buttons();

						return Cursor::State::IDLE;
					}
				}
			}
		}

		return UIElement::send_cursor(clicked, cursorpos);
	}

	void UIUserList::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (pressed)
		{
			if (escape)
				deactivate();
			else if (keycode == KeyAction::Id::TAB)
				change_tab((tab + 1) % Tab::NUM_TABS);
		}
	}

	UIElement::Type UIUserList::get_type() const
	{
		return TYPE;
	}

	void UIUserList::change_tab(uint16_t tabid)
	{
		buttons[Buttons::BT_TAB0 + tab]->set_state(Button::State::NORMAL);
		buttons[Buttons::BT_TAB0 + tabid]->set_state(Button::State::PRESSED);
		tab = tabid;

		for (uint16_t i = Buttons::BT_HP; i <= Buttons::BT_INVITE; i++)
			buttons[i]->set_active(tab == Tab::PARTY);

		update_buttons();
	}

	uint16_t UIUserList::get_tab()
	{
		return tab;
	}

	Button::State UIUserList::button_pressed(uint16_t buttonid)
	{
		const PartyMember* member = selected_member();

		switch (buttonid)
		{
			case Buttons::BT_CLOSE:
				deactivate();
				break;
			case Buttons::BT_HP:
				party.toggle_hp_marks();
				break;
			case Buttons::BT_CHANGE_LEADER:
				if (member)
					ChangePartyLeaderPacket(member->id).dispatch();

				break;
			case Buttons::BT_WHISPER:
				if (member)
					if (auto chatbar = UI::get().get_element<UIChatBar>())
						chatbar->start_whisper(member->name);

				break;
			case Buttons::BT_CHAT:
				if (auto chatbar = UI::get().get_element<UIChatBar>())
					chatbar->set_target(UIChatBar::Target::PARTY);

				break;
			case Buttons::BT_CREATE:
				CreatePartyPacket().dispatch();
				break;
			case Buttons::BT_LEAVE:
				LeavePartyPacket().dispatch();
				break;
			case Buttons::BT_EXPEL:
				if (member)
					ExpelFromPartyPacket(member->id).dispatch();

				break;
			case Buttons::BT_INVITE:
				UI::get().emplace<UIEnterText>(
						"Who do you want to invite?",
						[](const std::string& name)
						{
							InviteToPartyPacket(name).dispatch();
						},
						NAME_LIMIT
				);
				break;
			default:
				if (buttonid >= Buttons::BT_TAB0 && buttonid < Buttons::BT_TAB0 + Tab::NUM_TABS)
				{
					change_tab(buttonid - Buttons::BT_TAB0);

					return Button::State::PRESSED;
				}

				break;
		}

		return Button::State::NORMAL;
	}

	std::vector<UIUserList::Shown> UIUserList::currently_shown() const
	{
		std::vector<Shown> result;

		for (const PartyMember& member : party.get_members())
			result.push_back({ member.id, member.name, member.job, member.level, member.channel, member.map_id });

		return result;
	}

	void UIUserList::rebuild()
	{
		shown = currently_shown();
		shown_leader = party.get_leader();
		shown_mapid = Stage::get().get_mapid();

		if (!party.find(selected))
			selected = 0;

		std::vector<const PartyMember*> online_members, offline_members;

		for (const PartyMember& member : party.get_members())
			(member.online() ? online_members : offline_members).push_back(&member);

		// Footage counts only the members on the player's map as online,
		// though the others are listed, greyed.
		size_t nearby = 0;

		for (const PartyMember* member : online_members)
			if (member->map_id == shown_mapid)
				nearby++;

		size_t total = party.get_members().size();

		online.count.change_text(std::to_string(nearby) + "/" + std::to_string(total));
		offline.count.change_text(std::to_string(offline_members.size()) + "/" + std::to_string(total));

		int16_t next = lay_out(online, online_members, LIST.y());
		lay_out(offline, offline_members, next + SECTION_GAP);
	}

	int16_t UIUserList::lay_out(Section& section, const std::vector<const PartyMember*>& members, int16_t y)
	{
		section.top = y;
		section.lines.clear();

		y += HEADER + COLUMNS;

		for (const PartyMember* member : members)
		{
			Color::Name color = member->online() && member->map_id == shown_mapid
								? Color::Name::BLACK
								: Color::Name::SILVERCHALICE;

			section.lines.push_back({
					member->id,
					member->id == shown_leader,
					y,
					Text(Text::Font::A12M, Text::Alignment::LEFT, color, member->name),
					Text(Text::Font::A12M, Text::Alignment::LEFT, color, Job(member->job).get_name()),
					Text(Text::Font::A12M, Text::Alignment::LEFT, color, std::to_string(member->level))
			});

			y += ROW;
		}

		if (members.empty())
			y += ROW;

		y += FOOTER;
		section.bottom = y;

		return y;
	}

	void UIUserList::enable(uint16_t buttonid, bool enabled)
	{
		Button& button = *buttons[buttonid];
		bool is_enabled = button.get_state() != Button::State::DISABLED;

		// Leave the state alone otherwise, so mouseover survives.
		if (is_enabled != enabled)
			button.set_state(enabled ? Button::State::NORMAL : Button::State::DISABLED);
	}

	void UIUserList::update_buttons()
	{
		int32_t self = Stage::get().get_player().get_oid();
		bool in_party = party.exists();
		bool leader = in_party && party.is_leader(self);
		const PartyMember* member = selected_member();

		enable(Buttons::BT_HP, in_party);
		enable(Buttons::BT_CHANGE_LEADER, leader && member && member->online());
		enable(Buttons::BT_WHISPER, member && member->online());
		enable(Buttons::BT_CHAT, in_party);
		enable(Buttons::BT_SEARCH, false);
		enable(Buttons::BT_CREATE, !in_party);
		enable(Buttons::BT_LEAVE, in_party);
		enable(Buttons::BT_EXPEL, leader && member);
		enable(Buttons::BT_INVITE, leader && party.get_members().size() < PARTY_CAPACITY);
	}

	const PartyMember* UIUserList::selected_member() const
	{
		if (selected == 0 || selected == Stage::get().get_player().get_oid())
			return nullptr;

		return party.find(selected);
	}
}
