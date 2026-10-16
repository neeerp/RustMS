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

#include "../PartyCommands.h"
#include "../UI.h"

#include "../Components/AreaButton.h"
#include "../Components/MapleButton.h"

#include "../../Configuration.h"

#include "../../Character/Job.h"
#include "../../Gameplay/Stage.h"
#include "../../Util/NxFiles.h"

#include "../../Net/Packets/GameplayPackets.h"

#include <nlnx/nx.hpp>

#include <algorithm>

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
		// The buddy list shares the party list's corner, header, and rows,
		// but splits each row in two: same-channel buddies on the left and
		// the rest on the right, under a legend saying so.
		constexpr int16_t LEGEND = 19;
		constexpr int16_t BUDDY_FOOTER = 6;
		constexpr int16_t CELL_WIDTH = 140;
		constexpr int16_t CELL_NAME_X = 18;
		constexpr int16_t CELL_CHANNEL_X = 134;
		constexpr Point<int16_t> CELL_HIGHLIGHT = Point<int16_t>(CELL_WIDTH - 1, ROW);
		// The buddy buttons: one row, then INVITE TO PARTY and the filter.
		constexpr int16_t BUDDY_BUTTON_GAP = 7;
		constexpr int16_t BUTTONS_RIGHT = 299;
		const char* DEFAULT_GROUP = "Default Group";
	}

	bool UIUserList::Shown::operator==(const Shown& other) const
	{
		return id == other.id && name == other.name && job == other.job && level == other.level &&
			   channel == other.channel && map_id == other.map_id;
	}

	UIUserList::UIUserList(uint16_t t)
			: UIDragElement<PosUSERLIST>(), tab(t), party(Stage::get().get_player().get_party()),
			  buddies(Stage::get().get_player().get_buddies())
	{
		nl::node UserList = NxFiles::ui_classic["UIWindow.img"]["UserList"];
		nl::node Party = UserList["Party"];
		nl::node Friend = UserList["Friend"];
		nl::node backgrnd = UserList["backgrnd"];

		sprites.emplace_back(backgrnd);

		tabs = ClassicTabs(ClassicTabs::TAB5, UserList["Tab"]["enabled"], UserList["Tab"]["disabled"], TAB_FILL);

		buttons[Buttons::BT_CLOSE] = std::make_unique<MapleButton>(NxFiles::ui_classic["Basic.img"]["BtClose"], CLOSE);

		for (uint16_t i = 0; i < Tab::NUM_TABS; i++)
		{
			Point<int16_t> tab_pos = TAB_STRIP + Point<int16_t>(tabs.pitch() * i, 0);

			buttons[Buttons::BT_TAB0 + i] = std::make_unique<AreaButton>(tab_pos,
																		 Point<int16_t>(tabs.pitch(), tabs.height()));
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

		const char* buddy_buttons[] = { "BtAddFriend", "BtDelete", "BtWhisper", "BtChat", "BtWhere" };
		int16_t buddy_x = BUTTONS.x();

		for (uint16_t i = 0; i < std::size(buddy_buttons); i++)
		{
			nl::node button = Friend[buddy_buttons[i]];
			buttons[Buttons::BT_ADD_BUDDY + i] = std::make_unique<MapleButton>(button, Point<int16_t>(buddy_x, BUTTONS.y()));
			buddy_x += Texture(button["normal"]["0"]).get_dimensions().x() + BUDDY_BUTTON_GAP;
		}

		Point<int16_t> second_row = BUTTONS + Point<int16_t>(0, BUTTON_PITCH.y());
		buttons[Buttons::BT_INVITE_BUDDY] = std::make_unique<MapleButton>(Friend["BtParty"], second_row);

		for (auto [id, name] : { std::pair(Buttons::BT_SHOW_ONLINE, "BtShowOnline"), std::pair(Buttons::BT_SHOW_ALL, "BtShowAll") })
		{
			int16_t width = Texture(Friend[name]["normal"]["0"]).get_dimensions().x();
			buttons[id] = std::make_unique<MapleButton>(Friend[name], Point<int16_t>(BUTTONS_RIGHT - width, second_row.y()));
		}

		buddies_online.header = Friend["friend0"];
		buddies_offline.header = Friend["friend4"];
		legend = Friend["friend1"];
		buddy_row = Friend["friend2"];
		offline_row = Friend["friend5"];
		buddy_bottom = Friend["friend3"];
		buddies_online.count = Text(Text::Font::A11M, Text::Alignment::RIGHT, Color::Name::WHITE);
		buddies_offline.count = Text(Text::Font::A11M, Text::Alignment::RIGHT, Color::Name::WHITE);

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
		rebuild_buddies();
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
		else if (tab == Tab::FRIEND)
		{
			draw_buddies(buddies_online, buddy_row, true);

			if (show_all)
				draw_buddies(buddies_offline, offline_row, false);
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

	void UIUserList::draw_buddies(const BuddySection& section, const Texture& section_row, bool with_legend) const
	{
		Point<int16_t> top = position + Point<int16_t>(LIST.x(), section.top);

		section.header.draw(top);
		section.count.draw(top + COUNT);

		int16_t first = HEADER;

		if (with_legend)
		{
			legend.draw(top + Point<int16_t>(0, HEADER));
			first += LEGEND;
		}

		for (const Cell& cell : section.cells)
			if (cell.id == picked_buddy)
				highlight.draw(DrawArgument(position + cell.pos + Point<int16_t>(1, 0), CELL_HIGHLIGHT));

		for (int16_t i = 0; i < section.rows; i++)
			section_row.draw(top + Point<int16_t>(0, first + ROW * i));

		for (const Cell& cell : section.cells)
		{
			cell.name.draw(position + cell.pos + Point<int16_t>(CELL_NAME_X, TEXT_Y));
			cell.channel.draw(position + cell.pos + Point<int16_t>(CELL_CHANNEL_X, TEXT_Y));
		}

		buddy_bottom.draw(position + Point<int16_t>(LIST.x(), section.bottom - BUDDY_FOOTER));
	}

	void UIUserList::update()
	{
		UIElement::update();

		int32_t mapid = Stage::get().get_mapid();
		std::vector<Shown> now = currently_shown();

		if (now != shown || party.get_leader() != shown_leader || mapid != shown_mapid)
			rebuild();

		if (buddies.get_buddies() != shown_buddies)
			rebuild_buddies();

		update_buttons();
	}

	Cursor::State UIUserList::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		Cursor::State dstate = UIDragElement::send_cursor(clicked, cursorpos);

		if (dragged)
			return dstate;

		if (!clicked)
			filter_clicked = false;

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

		if (clicked && tab == Tab::FRIEND)
		{
			for (const BuddySection* section : {&buddies_online, &buddies_offline})
			{
				if (section == &buddies_offline && !show_all)
					continue;

				for (const Cell& cell : section->cells)
				{
					Point<int16_t> corner = cell.pos + Point<int16_t>(1, 0);

					if (Rectangle<int16_t>(corner, corner + CELL_HIGHLIGHT).contains(cursor_relative))
					{
						picked_buddy = cell.id;
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

		for (uint16_t i = Buttons::BT_ADD_BUDDY; i <= Buttons::BT_INVITE_BUDDY; i++)
			buttons[i]->set_active(tab == Tab::FRIEND);

		// The filter button offers whichever view isn't showing.
		buttons[Buttons::BT_SHOW_ONLINE]->set_active(tab == Tab::FRIEND && show_all);
		buttons[Buttons::BT_SHOW_ALL]->set_active(tab == Tab::FRIEND && !show_all);

		update_buttons();
	}

	uint16_t UIUserList::get_tab()
	{
		return tab;
	}

	Button::State UIUserList::button_pressed(uint16_t buttonid)
	{
		const PartyMember* member = selected_member();
		const Buddy* buddy = selected_buddy();

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
				PartyCommands::create();
				break;
			case Buttons::BT_LEAVE:
				PartyCommands::leave();
				break;
			case Buttons::BT_EXPEL:
				if (member)
					PartyCommands::expel(member->name);

				break;
			case Buttons::BT_INVITE:
				UI::get().emplace<UIEnterText>(
						"Who do you want to invite?",
						[](const std::string& name)
						{
							PartyCommands::invite(name);
						},
						NAME_LIMIT
				);
				break;
			case Buttons::BT_ADD_BUDDY:
				UI::get().emplace<UIEnterText>(
						"Enter the character name you'd like to add as a buddy.",
						[](const std::string& name)
						{
							AddBuddyPacket(name, DEFAULT_GROUP).dispatch();
						},
						NAME_LIMIT
				);
				break;
			case Buttons::BT_DELETE_BUDDY:
				if (buddy)
				{
					int32_t id = buddy->id;

					UI::get().emplace<UIYesNo>(
							"Are you sure you want to delete '" + buddy->name + "'\\nfrom your buddy list?",
							[id](bool yes)
							{
								if (yes)
									DeleteBuddyPacket(id).dispatch();
							}
					);
				}

				break;
			case Buttons::BT_WHISPER_BUDDY:
				if (buddy)
					if (auto chatbar = UI::get().get_element<UIChatBar>())
						chatbar->start_whisper(buddy->name);

				break;
			case Buttons::BT_BUDDY_CHAT:
				if (auto chatbar = UI::get().get_element<UIChatBar>())
					chatbar->set_target(UIChatBar::Target::BUDDY);

				break;
			case Buttons::BT_INVITE_BUDDY:
				if (buddy)
					PartyCommands::invite(buddy->name);

				break;
			case Buttons::BT_SHOW_ONLINE:
			case Buttons::BT_SHOW_ALL:
				// The two share a spot, and one click can reach both: the
				// cursor keeps clicking until the button is released.
				if (!filter_clicked)
				{
					filter_clicked = true;
					show_all = buttonid == Buttons::BT_SHOW_ALL;
					change_tab(tab);
				}

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

	void UIUserList::rebuild_buddies()
	{
		shown_buddies = buddies.get_buddies();

		if (!buddies.find(picked_buddy))
			picked_buddy = 0;

		int32_t channel = Configuration::get().get_channelid();
		std::vector<const Buddy*> same, other, away;

		for (const Buddy& buddy : shown_buddies)
		{
			if (!buddy.online())
				away.push_back(&buddy);
			else if (buddy.channel == channel)
				same.push_back(&buddy);
			else
				other.push_back(&buddy);
		}

		std::string total = "/" + std::to_string(shown_buddies.size());
		buddies_online.count.change_text(std::to_string(same.size() + other.size()) + total);
		buddies_offline.count.change_text(std::to_string(away.size()) + total);

		auto cell = [](const Buddy& buddy, Point<int16_t> pos, bool online)
		{
			Color::Name color = online ? Color::Name::BLACK : Color::Name::SILVERCHALICE;

			return Cell{ buddy.id, pos, Text(Text::Font::A12M, Text::Alignment::LEFT, color, buddy.name), Text() };
		};

		// Online: a column per kind, as long as the longer one.
		buddies_online.top = LIST.y();
		buddies_online.cells.clear();
		int16_t first = buddies_online.top + HEADER + LEGEND;

		for (size_t i = 0; i < same.size(); i++)
			buddies_online.cells.push_back(cell(*same[i], Point<int16_t>(LIST.x(), first + ROW * i), true));

		for (size_t i = 0; i < other.size(); i++)
		{
			Cell c = cell(*other[i], Point<int16_t>(LIST.x() + CELL_WIDTH, first + ROW * i), true);
			c.channel = Text(Text::Font::A11M, Text::Alignment::RIGHT, Color::Name::SILVERCHALICE,
							 "Ch. " + std::to_string(other[i]->channel + 1));
			buddies_online.cells.push_back(std::move(c));
		}

		buddies_online.rows = static_cast<int16_t>(std::max<size_t>({ 1, same.size(), other.size() }));
		buddies_online.bottom = first + ROW * buddies_online.rows + BUDDY_FOOTER;

		// Offline: left to right, then down.
		buddies_offline.top = buddies_online.bottom + SECTION_GAP;
		buddies_offline.cells.clear();
		first = buddies_offline.top + HEADER;

		for (size_t i = 0; i < away.size(); i++)
		{
			Point<int16_t> pos = Point<int16_t>(LIST.x() + CELL_WIDTH * (i % 2), first + ROW * (i / 2));
			buddies_offline.cells.push_back(cell(*away[i], pos, false));
		}

		buddies_offline.rows = static_cast<int16_t>(std::max<size_t>(1, (away.size() + 1) / 2));
		buddies_offline.bottom = first + ROW * buddies_offline.rows + BUDDY_FOOTER;
	}

	const Buddy* UIUserList::selected_buddy() const
	{
		return picked_buddy == 0 ? nullptr : buddies.find(picked_buddy);
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

		const Buddy* buddy = selected_buddy();
		bool party_has_room = !in_party || (leader && party.get_members().size() < PARTY_CAPACITY);

		enable(Buttons::BT_ADD_BUDDY, !buddies.is_full());
		enable(Buttons::BT_DELETE_BUDDY, buddy);
		enable(Buttons::BT_WHISPER_BUDDY, buddy && buddy->online());
		enable(Buttons::BT_BUDDY_CHAT, !buddies.get_buddies().empty());
		enable(Buttons::BT_BUDDY_MAP, false);
		// Inviting someone makes a party first if the player has none.
		enable(Buttons::BT_INVITE_BUDDY, buddy && buddy->online() && party_has_room);
	}

	const PartyMember* UIUserList::selected_member() const
	{
		if (selected == 0 || selected == Stage::get().get_player().get_oid())
			return nullptr;

		return party.find(selected);
	}
}
