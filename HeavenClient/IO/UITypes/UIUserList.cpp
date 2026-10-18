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
#include "../../Constants.h"

#include "../../Character/Job.h"
#include "../../Gameplay/Stage.h"
#include "../../Graphics/GraphicsGL.h"
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

		// The guild tab, as the client lays it out (FUN_0090f17c). The
		// client places text by its top; HeavenClient's 5 pixels lower.
		constexpr int16_t CLIENT_TEXT = -5;
		// The header: the guild's emblem and name, centred, then the notice.
		constexpr Point<int16_t> GUILD_HEADER = Point<int16_t>(5, 55);
		constexpr int16_t GUILD_HEADER_WIDTH = 306;
		constexpr int16_t EMBLEM_SPACE = 26;
		constexpr int16_t GUILD_NAME_Y = 3;
		constexpr Point<int16_t> NOTICE_BOX = Point<int16_t>(5, 80);
		constexpr Point<int16_t> NOTICE_LABEL = Point<int16_t>(13, 85);
		// A notice scrolls left through its strip a pixel each 30 ms update,
		// and starts over once it has left (FUN_00910dde).
		constexpr Point<int16_t> TICKER = Point<int16_t>(10, 85);
		constexpr int16_t TICKER_WIDTH = 270;
		constexpr int16_t TICKER_START = 250;
		constexpr uint16_t TICKER_STEP = 30;
		// The member list scrolls through its view 33 pixels a step.
		constexpr Point<int16_t> GUILD_LIST = Point<int16_t>(6, 113);
		constexpr int16_t GUILD_VIEW = 192;
		constexpr int16_t GUILD_STEP = 33;
		constexpr Range<int16_t> GUILD_SLIDER = Range<int16_t>(113, 292);
		constexpr int16_t GUILD_SLIDER_X = 290;
		// Each section: a header, then, unless folded, column headings, a
		// page of rows (or "none") and an end line.
		constexpr int16_t GUILD_SECTION_HEADER = 27;
		constexpr int16_t GUILD_COLUMNS_GAP = 4;
		constexpr int16_t GUILD_END = 1;
		constexpr int16_t GUILD_SECTION_GAP = 10;
		constexpr int16_t MEMBERS_PER_PAGE = 20;
		// In a section's header.
		constexpr Point<int16_t> GUILD_COUNT = Point<int16_t>(185, 6);
		constexpr Point<int16_t> GUILD_PAGE = Point<int16_t>(261, 6);
		constexpr Point<int16_t> GUILD_FOLD = Point<int16_t>(172, 8);
		constexpr int16_t PAGE_BUTTON = 13;
		// In a member's row.
		constexpr int16_t MEMBER_NAME_X = 10;
		constexpr int16_t MEMBER_JOB_X = 100;
		constexpr int16_t MEMBER_LEVEL_X = 170;
		constexpr int16_t MEMBER_TITLE_X = 210;
		constexpr int16_t MEMBER_TEXT_Y = 4;
		constexpr int16_t MEMBER_JOB_WIDTH = 67;
		constexpr int16_t GUILD_ROW_WIDTH = 281;
		// A picked row is filled from its second column (0xFF244768).
		constexpr int16_t PICKED_LEFT = 1;
		constexpr int16_t PICKED_WIDTH = GUILD_ROW_WIDTH - PICKED_LEFT;
		constexpr float PICKED_RED = 0x24 / 255.0f;
		constexpr float PICKED_GREEN = 0x47 / 255.0f;
		constexpr float PICKED_BLUE = 0x68 / 255.0f;
		// Every guild button shows on the tab; ranks only grey them out.
		// In the order of the guild buttons in Buttons.
		const std::pair<const char*, Point<int16_t>> GUILD_BUTTONS[] = {
				{ "BtInvite", Point<int16_t>(9, 316) },
				{ "BtWithdraw", Point<int16_t>(80, 316) },
				{ "BtKick", Point<int16_t>(159, 336) },
				{ "BtUp", Point<int16_t>(9, 336) },
				{ "BtDown", Point<int16_t>(80, 336) },
				{ "BtWhere", Point<int16_t>(159, 316) },
				{ "BtChat", Point<int16_t>(238, 336) },
				{ "BtInfo", Point<int16_t>(238, 363) },
				{ "BtWhisper", Point<int16_t>(238, 316) },
				{ "Btnotice", Point<int16_t>(287, 80) },
				{ "BtPartyInvite", Point<int16_t>(159, 363) },
				{ "BtGuildBBS", Point<int16_t>(80, 363) }
		};
		// A guild section's buttons, from BT_GUILD_SECTION.
		enum SectionButton : uint16_t
		{
			FOLD,
			UNFOLD,
			PREVIOUS_PAGE,
			NEXT_PAGE,
			SECTION_BUTTONS
		};

		// `text` cut to `width`, with `tail` after it if cut.
		std::string fit(Text::Font font, const std::string& text, int16_t width, const std::string& tail)
		{
			Text measure(font, Text::Alignment::LEFT, Color::Name::BLACK, text, 0, false);

			if (measure.width() <= width)
				return text;

			std::string cut = text;

			while (!cut.empty())
			{
				cut.pop_back();
				measure.change_text(cut + tail);

				if (measure.width() <= width)
					break;
			}

			return cut + tail;
		}

		std::string fraction(size_t part, size_t whole)
		{
			return "[" + std::to_string(part) + "/" + std::to_string(whole) + "]";
		}
	}

	bool UIUserList::Shown::operator==(const Shown& other) const
	{
		return id == other.id && name == other.name && job == other.job && level == other.level &&
			   channel == other.channel && map_id == other.map_id;
	}

	UIUserList::UIUserList(uint16_t t)
			: UIDragElement<PosUSERLIST>(), tab(t), party(Stage::get().get_player().get_party()),
			  buddies(Stage::get().get_player().get_buddies()), guild(Stage::get().get_player().get_guild())
	{
		nl::node UserList = NxFiles::ui_classic["UIWindow.img"]["UserList"];
		nl::node Party = UserList["Party"];
		nl::node Friend = UserList["Friend"];
		nl::node GuildInfo = UserList["Guild"]["GuildInfo"];
		nl::node backgrnd = UserList["backgrnd"];

		background = backgrnd;
		guild_background = UserList["backgrnd2"];

		tabs = ClassicTabs(ClassicTabs::TAB5, UserList["Tab"]["enabled"], UserList["Tab"]["disabled"], TAB_FILL);

		buttons[Buttons::BT_CLOSE] = std::make_unique<MapleButton>(NxFiles::ui_classic["Basic.img"]["BtClose"], CLOSE);

		for (uint16_t i = 0; i < Tab::NUM_TABS; i++)
		{
			Point<int16_t> tab_pos = TAB_STRIP + Point<int16_t>(tabs.pitch() * i, 0);

			buttons[Buttons::BT_TAB0 + i] = std::make_unique<AreaButton>(tab_pos,
																		 Point<int16_t>(tabs.pitch(), tabs.height()));
			buttons[Buttons::BT_TAB0 + i]->set_tab();
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

		for (uint16_t i = 0; i < std::size(GUILD_BUTTONS); i++)
		{
			// The client places these by their top left, whatever the art's
			// origin, which only BtGuildBBS and BtInfo set.
			auto [name, pos] = GUILD_BUTTONS[i];
			nl::node button = GuildInfo[name];
			buttons[Buttons::BT_GUILD_INVITE + i] = std::make_unique<MapleButton>(button,
																				   pos + Texture(button["normal"]["0"]).get_origin());
		}

		nl::node Basic = NxFiles::ui_classic["Basic.img"];
		nl::node itemSearch = NxFiles::ui_classic["UIWindow.img"]["itemSearch"];

		for (uint16_t s = 0; s < 2; s++)
		{
			uint16_t first = Buttons::BT_GUILD_SECTION + s * SectionButton::SECTION_BUTTONS;

			buttons[first + SectionButton::FOLD] = std::make_unique<MapleButton>(Basic["BtMin"]);
			buttons[first + SectionButton::UNFOLD] = std::make_unique<MapleButton>(Basic["BtMax"]);
			buttons[first + SectionButton::PREVIOUS_PAGE] = std::make_unique<MapleButton>(itemSearch["BtLeft"]);
			buttons[first + SectionButton::NEXT_PAGE] = std::make_unique<MapleButton>(itemSearch["BtRight"]);
		}

		guild_online.header = GuildInfo["guildinfo0"];
		guild_offline.header = GuildInfo["guildinfo1"];
		guild_columns = GuildInfo["guildinfo2"];
		guild_row = GuildInfo["guildinfo3"];
		guild_end = GuildInfo["guildinfo4"];
		notice_box = GuildInfo["guildinfo6"];
		guild_frame = GuildInfo["guildmark"];

		for (GuildSection* section : { &guild_online, &guild_offline })
		{
			section->count = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE);
			section->page_text = Text(Text::Font::A12M, Text::Alignment::RIGHT, Color::Name::WHITE);
		}

		guild_name = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK);
		no_guild = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, "You are not in the guild.");
		notice_label = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, "* Guild Notice");
		notice = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, "", 0, false);

		guild_slider = Slider(Slider::Type::CLASSIC, GUILD_SLIDER, GUILD_SLIDER_X, 1, 0,
							  [&](bool up) { scroll_guild(guild_scroll + (up ? -1 : 1)); });

		dimension = Texture(backgrnd).get_dimensions();
		dragarea = Point<int16_t>(dimension.x(), 20);

		rebuild();
		rebuild_buddies();
		rebuild_guild();
		change_tab(tab);
	}

	void UIUserList::draw(float alpha) const
	{
		bool guild_art = tab == Tab::GUILD || tab == Tab::ALLIANCE;
		(guild_art ? guild_background : background).draw(position);

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
		else if (tab == Tab::GUILD)
		{
			draw_guild();
		}

		UIElement::draw_buttons(alpha);
	}

	void UIUserList::draw_guild() const
	{
		Point<int16_t> header = position + GUILD_HEADER;

		if (guild.exists())
		{
			int16_t x = (GUILD_HEADER_WIDTH - guild_name.width() - EMBLEM_SPACE) / 2;

			if (emblem_marked)
				emblem.draw(header + Point<int16_t>(x, 1));
			else
				guild_frame.draw(header + Point<int16_t>(x, 0));

			guild_name.draw(header + Point<int16_t>(x + EMBLEM_SPACE, GUILD_NAME_Y + CLIENT_TEXT));
		}
		else
		{
			int16_t x = (GUILD_HEADER_WIDTH - no_guild.width()) / 2;
			no_guild.draw(header + Point<int16_t>(x, GUILD_NAME_Y + CLIENT_TEXT));
		}

		notice_box.draw(position + NOTICE_BOX);

		if (guild.exists())
		{
			if (shown_guild.notice.empty())
			{
				notice_label.draw(position + NOTICE_LABEL + Point<int16_t>(0, CLIENT_TEXT));
			}
			else
			{
				Point<int16_t> strip = position + TICKER;
				notice.draw_between(strip + Point<int16_t>(ticker_x, CLIENT_TEXT),
									Range<int16_t>(strip.x(), strip.x() + TICKER_WIDTH));
			}
		}

		// The list is laid out in its own space, from 0 down, and shows
		// through a view the slider moves.
		int16_t view_top = guild_scroll * GUILD_STEP;
		int16_t view_bottom = view_top + GUILD_VIEW;
		Point<int16_t> list = position + GUILD_LIST - Point<int16_t>(0, view_top);
		Range<int16_t> screen_rows = Range<int16_t>(position.y() + GUILD_LIST.y(),
													position.y() + GUILD_LIST.y() + GUILD_VIEW);

		auto art = [&](const Texture& texture, int16_t y)
		{
			int16_t from = std::max<int16_t>(0, view_top - y);
			int16_t to = std::min<int16_t>(texture.height(), view_bottom - y);

			if (from < to)
				texture.draw(list + Point<int16_t>(0, y), Rectangle<int16_t>(0, texture.width(), from, to));
		};

		auto words = [&](const Text& text, int16_t x, int16_t y)
		{
			if (y + text.height() > view_top && y < view_bottom)
				text.draw(list + Point<int16_t>(x, y + CLIENT_TEXT), screen_rows);
		};

		for (const GuildSection* section : { &guild_online, &guild_offline })
		{
			art(section->header, section->top);
			words(section->count, GUILD_COUNT.x(), section->top + GUILD_COUNT.y());
			words(section->page_text, GUILD_PAGE.x(), section->top + GUILD_PAGE.y());

			if (!section->expanded)
				continue;

			art(guild_columns, section->first - COLUMNS);

			if (section->rows.empty())
			{
				art(guild_row, section->first);
				words(none, MEMBER_NAME_X, section->first + MEMBER_TEXT_Y);
			}

			for (const GuildRow& row : section->rows)
			{
				art(guild_row, row.y);

				if (row.id == guild_picked)
				{
					int16_t top = std::max(row.y, view_top);
					int16_t bottom = std::min<int16_t>(row.y + ROW, view_bottom);

					if (top < bottom)
						GraphicsGL::get().drawrectangle(list.x() + PICKED_LEFT, list.y() + top, PICKED_WIDTH,
														bottom - top, PICKED_RED, PICKED_GREEN, PICKED_BLUE, 1.0f);
				}

				words(row.name, MEMBER_NAME_X, row.y + MEMBER_TEXT_Y);
				words(row.job, MEMBER_JOB_X, row.y + MEMBER_TEXT_Y);
				words(row.level, MEMBER_LEVEL_X, row.y + MEMBER_TEXT_Y);
				words(row.title, MEMBER_TITLE_X, row.y + MEMBER_TEXT_Y);
			}

			art(guild_end, section->end);
		}

		guild_slider.draw(position);
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

		if (!(guild.get_data() == shown_guild))
			rebuild_guild();

		if (guild.exists() && !shown_guild.notice.empty())
		{
			ticker_time += Constants::TIMESTEP;

			for (; ticker_time >= TICKER_STEP; ticker_time -= TICKER_STEP)
				if (--ticker_x + notice.width() < 0)
					ticker_x = TICKER_START;
		}

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

		if (tab == Tab::GUILD)
		{
			if (Cursor::State state = guild_slider.send_cursor(cursor_relative, clicked))
				return state;

			Point<int16_t> in_view = cursor_relative - GUILD_LIST;

			if (clicked && in_view.x() >= 0 && in_view.x() < GUILD_ROW_WIDTH && in_view.y() >= 0 &&
				in_view.y() < GUILD_VIEW)
			{
				int16_t y = in_view.y() + guild_scroll * GUILD_STEP;

				for (const GuildSection* section : { &guild_online, &guild_offline })
				{
					for (const GuildRow& row : section->rows)
					{
						if (y >= row.y && y < row.y + ROW)
						{
							guild_picked = row.id;
							rebuild_guild();
							update_buttons();

							return Cursor::State::IDLE;
						}
					}
				}
			}
		}

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

	void UIUserList::send_scroll(double yoffset)
	{
		if (tab == Tab::GUILD)
			guild_slider.send_scroll(yoffset);
	}

	UIElement::Type UIUserList::get_type() const
	{
		return TYPE;
	}

	void UIUserList::change_tab(uint16_t tabid)
	{
		buttons[Buttons::BT_TAB0 + tab]->set_state(Button::State::NORMAL);
		buttons[Buttons::BT_TAB0 + tabid]->set_state(Button::State::PRESSED);

		if (tabid != tab)
			guild_picked = 0;

		tab = tabid;

		for (uint16_t i = Buttons::BT_HP; i <= Buttons::BT_INVITE; i++)
			buttons[i]->set_active(tab == Tab::PARTY);

		for (uint16_t i = Buttons::BT_ADD_BUDDY; i <= Buttons::BT_INVITE_BUDDY; i++)
			buttons[i]->set_active(tab == Tab::FRIEND);

		// The filter button offers whichever view isn't showing.
		buttons[Buttons::BT_SHOW_ONLINE]->set_active(tab == Tab::FRIEND && show_all);
		buttons[Buttons::BT_SHOW_ALL]->set_active(tab == Tab::FRIEND && !show_all);

		for (uint16_t i = Buttons::BT_GUILD_INVITE; i <= Buttons::BT_GUILD_BBS; i++)
			buttons[i]->set_active(tab == Tab::GUILD);

		// Also shows or hides the section buttons.
		rebuild_guild();
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
				{
					int32_t id = member->id;

					UI::get().emplace<UIYesNo>(
							"Do you wish to pass the leader position to \\n" + member->name + "?",
							[id](bool yes)
							{
								if (yes)
									ChangePartyLeaderPacket(id).dispatch();
							}
					);
				}

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
						"Please enter the name of the character\\nyou'll invite to your party.",
						[](const std::string& name)
						{
							PartyCommands::invite(name);
						},
						NAME_LIMIT
				);
				break;
			case Buttons::BT_ADD_BUDDY:
				UI::get().emplace<UIEnterText>(
						"Please enter the name of the character\\nyou'll add as a buddy.",
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
							"Are you sure you want to delete\\n'" + buddy->name + "' from the list?",
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
			case Buttons::BT_GUILD_WHISPER:
				if (const GuildMember* picked = picked_online_member())
					if (auto chatbar = UI::get().get_element<UIChatBar>())
						chatbar->start_whisper(picked->name);

				break;
			case Buttons::BT_GUILD_CHAT:
				if (auto chatbar = UI::get().get_element<UIChatBar>())
					chatbar->set_target(UIChatBar::Target::GUILD);

				break;
			case Buttons::BT_GUILD_PARTY_INVITE:
				if (const GuildMember* picked = picked_online_member())
					PartyCommands::invite(picked->name);

				break;
			default:
				if (buttonid >= Buttons::BT_GUILD_SECTION && buttonid < Buttons::BT_TAB0)
				{
					uint16_t index = buttonid - Buttons::BT_GUILD_SECTION;
					GuildSection& section = index < SectionButton::SECTION_BUTTONS ? guild_online : guild_offline;

					switch (index % SectionButton::SECTION_BUTTONS)
					{
						case SectionButton::FOLD:
						case SectionButton::UNFOLD:
							section.expanded = !section.expanded;
							break;
						case SectionButton::PREVIOUS_PAGE:
							section.page--;
							break;
						case SectionButton::NEXT_PAGE:
							section.page++;
							break;
					}

					rebuild_guild();

					// The last page disables its own arrow.
					if (buttons[buttonid]->get_state() == Button::State::DISABLED)
						return Button::State::DISABLED;
				}
				else if (buttonid >= Buttons::BT_TAB0 && buttonid < Buttons::BT_TAB0 + Tab::NUM_TABS)
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

		// The client's rules for the rest (FUN_0090f17c) need guild
		// operations the server doesn't have yet.
		const GuildMember* guild_member = picked_online_member();

		for (uint16_t id : { Buttons::BT_GUILD_INVITE, Buttons::BT_GUILD_WITHDRAW, Buttons::BT_GUILD_KICK,
							 Buttons::BT_GUILD_UP, Buttons::BT_GUILD_DOWN, Buttons::BT_GUILD_WHERE,
							 Buttons::BT_GUILD_INFO, Buttons::BT_GUILD_NOTICE, Buttons::BT_GUILD_BBS })
			enable(id, false);

		enable(Buttons::BT_GUILD_CHAT, guild_member);
		enable(Buttons::BT_GUILD_WHISPER, guild_member);
		enable(Buttons::BT_GUILD_PARTY_INVITE, guild_member && leader);
	}

	const PartyMember* UIUserList::selected_member() const
	{
		if (selected == 0 || selected == Stage::get().get_player().get_oid())
			return nullptr;

		return party.find(selected);
	}

	void UIUserList::rebuild_guild()
	{
		const GuildData& data = guild.get_data();

		if (data.notice != shown_guild.notice)
		{
			ticker_x = TICKER_START;
			ticker_time = 0;
		}

		shown_guild = data;

		if (!guild.find(guild_picked))
			guild_picked = 0;

		guild_name.change_text(shown_guild.name);
		emblem = GuildMark(shown_guild.emblem);
		// The client draws the emblem only with a mark, and the empty frame
		// otherwise.
		emblem_marked = shown_guild.emblem.mark != 0;
		notice.change_text(shown_guild.notice);

		std::vector<const GuildMember*> online_members, offline_members;

		for (const GuildMember& member : shown_guild.members)
			(member.online ? online_members : offline_members).push_back(&member);

		size_t total = shown_guild.members.size();
		guild_online.count.change_text(fraction(online_members.size(), total));
		guild_offline.count.change_text(fraction(offline_members.size(), total));

		int16_t next = lay_out(guild_online, online_members, 0);
		guild_height = lay_out(guild_offline, offline_members, next + GUILD_SECTION_GAP);

		scroll_guild(guild_scroll);
	}

	int16_t UIUserList::lay_out(GuildSection& section, const std::vector<const GuildMember*>& members, int16_t y)
	{
		int16_t count = static_cast<int16_t>(members.size());

		section.top = y;
		section.pages = std::max<int16_t>(1, (count + MEMBERS_PER_PAGE - 1) / MEMBERS_PER_PAGE);
		section.page = std::clamp<int16_t>(section.page, 1, section.pages);
		section.page_text.change_text(fraction(section.page, section.pages));
		section.rows.clear();

		y += GUILD_SECTION_HEADER;

		if (!section.expanded)
			return y;

		y += GUILD_COLUMNS_GAP + COLUMNS;
		section.first = y;

		int16_t last = std::min<int16_t>(count, section.page * MEMBERS_PER_PAGE);

		for (int16_t i = (section.page - 1) * MEMBERS_PER_PAGE; i < last; i++)
		{
			const GuildMember& member = *members[i];
			Color::Name color = member.id == guild_picked ? Color::Name::WHITE : Color::Name::BLACK;
			std::string job = member.job < 0
							  ? "??"
							  : fit(Text::Font::A12M, Job(static_cast<uint16_t>(member.job)).get_name(),
									MEMBER_JOB_WIDTH, "..");
			std::string level = member.level < 0 ? "??" : std::to_string(member.level);

			section.rows.push_back({
					member.id,
					y,
					Text(Text::Font::A12M, Text::Alignment::LEFT, color, member.name),
					Text(Text::Font::A12M, Text::Alignment::LEFT, color, job),
					Text(Text::Font::A12M, Text::Alignment::LEFT, color, level),
					Text(Text::Font::A12M, Text::Alignment::LEFT, color, guild.title(member.rank))
			});

			y += ROW;
		}

		if (count == 0)
			y += ROW;

		section.end = y;

		return y + GUILD_END;
	}

	void UIUserList::place_section_buttons()
	{
		int16_t view_top = guild_scroll * GUILD_STEP;
		Point<int16_t> list = GUILD_LIST - Point<int16_t>(0, view_top);
		const GuildSection* sections[] = { &guild_online, &guild_offline };

		for (uint16_t s = 0; s < 2; s++)
		{
			const GuildSection& section = *sections[s];
			uint16_t first = Buttons::BT_GUILD_SECTION + s * SectionButton::SECTION_BUTTONS;
			// The client makes them while the section's header is in view
			// (FUN_0091e88e). The page buttons flank the page number.
			bool shown = tab == Tab::GUILD && section.top >= view_top &&
						 section.top + GUILD_SECTION_HEADER <= view_top + GUILD_VIEW;
			Point<int16_t> top = list + Point<int16_t>(0, section.top);
			int16_t page_left = GUILD_PAGE.x() - section.page_text.width() - PAGE_BUTTON;

			buttons[first + SectionButton::FOLD]->set_position(top + GUILD_FOLD);
			buttons[first + SectionButton::UNFOLD]->set_position(top + GUILD_FOLD);
			buttons[first + SectionButton::PREVIOUS_PAGE]->set_position(top + Point<int16_t>(page_left, GUILD_PAGE.y()));
			buttons[first + SectionButton::NEXT_PAGE]->set_position(top + GUILD_PAGE);

			buttons[first + SectionButton::FOLD]->set_active(shown && section.expanded);
			buttons[first + SectionButton::UNFOLD]->set_active(shown && !section.expanded);
			buttons[first + SectionButton::PREVIOUS_PAGE]->set_active(shown);
			buttons[first + SectionButton::NEXT_PAGE]->set_active(shown);

			enable(first + SectionButton::PREVIOUS_PAGE, section.page > 1);
			enable(first + SectionButton::NEXT_PAGE, section.page < section.pages);
		}
	}

	void UIUserList::scroll_guild(int16_t scroll)
	{
		// The client gives the slider t / 33 + 2 positions for t pixels too
		// many (FUN_0091e1eb), enough to bring the list's end into view.
		int16_t overflow = guild_height - GUILD_VIEW;
		int16_t last = overflow > 0 ? overflow / GUILD_STEP + 1 : 0;

		guild_scroll = std::clamp<int16_t>(scroll, 0, last);
		guild_slider.setrows(guild_scroll, 1, last + 1);
		place_section_buttons();
	}

	const GuildMember* UIUserList::picked_online_member() const
	{
		if (guild_picked == 0 || guild_picked == Stage::get().get_player().get_oid())
			return nullptr;

		// A row counts only while the list shows it: not on another page or
		// in a folded section.
		for (const GuildSection* section : { &guild_online, &guild_offline })
		{
			for (const GuildRow& row : section->rows)
			{
				if (row.id == guild_picked)
				{
					const GuildMember* member = guild.find(row.id);

					return member && member->online ? member : nullptr;
				}
			}
		}

		return nullptr;
	}
}
