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
#include <cstdint>
#include <string>
#include <vector>

#include "../UIDragElement.h"

#include "../Components/ClassicTabs.h"
#include "../Components/Slider.h"

#include "../../Character/BuddyList.h"
#include "../../Character/Guild.h"
#include "../../Character/GuildMark.h"
#include "../../Character/Party.h"

namespace ms
{
	// The v83 Maple User List & Guild window (UIWindow.img/UserList). The
	// Friend, Party and Guild tabs work so far; the others show an empty
	// window.
	class UIUserList : public UIDragElement<PosUSERLIST>
	{
	public:
		static constexpr Type TYPE = UIElement::Type::USERLIST;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = true;

		enum Tab : uint16_t
		{
			FRIEND,
			PARTY,
			GUILD,
			ALLIANCE,
			BLACKLIST,
			NUM_TABS
		};

		UIUserList(uint16_t tab);

		void draw(float inter) const override;

		void update() override;

		// Closing the window closes the rank panel with it.
		void toggle_active() override;

		bool is_in_range(Point<int16_t> cursorpos) const override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		void send_scroll(double yoffset) override;

		void doubleclick(Point<int16_t> cursorpos) override;

		UIElement::Type get_type() const override;

		void change_tab(uint16_t tabid);

		uint16_t get_tab();

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		enum Buttons : uint16_t
		{
			BT_CLOSE,
			BT_HP,
			BT_CHANGE_LEADER,
			BT_WHISPER,
			BT_CHAT,
			BT_SEARCH,
			BT_CREATE,
			BT_LEAVE,
			BT_EXPEL,
			BT_INVITE,
			BT_ADD_BUDDY,
			BT_DELETE_BUDDY,
			BT_WHISPER_BUDDY,
			BT_BUDDY_CHAT,
			BT_BUDDY_MAP,
			BT_INVITE_BUDDY,
			BT_MESSAGE_BUDDY,
			BT_SHOW_ONLINE,
			BT_SHOW_ALL,
			BT_GUILD_INVITE,
			BT_GUILD_WITHDRAW,
			BT_GUILD_KICK,
			BT_GUILD_UP,
			BT_GUILD_DOWN,
			BT_GUILD_WHERE,
			BT_GUILD_CHAT,
			BT_GUILD_INFO,
			BT_GUILD_WHISPER,
			BT_GUILD_NOTICE,
			BT_GUILD_PARTY_INVITE,
			BT_GUILD_BBS,
			// The rank panel's.
			BT_GRADE_MODIFY,
			BT_GRADE_SAVE,
			// Each guild section's fold, unfold, previous and next page
			// buttons: the online section's, then the offline one's.
			BT_GUILD_SECTION,
			BT_TAB0 = BT_GUILD_SECTION + 8
		};

		// One member's row; y is relative to the window.
		struct Line
		{
			int32_t id;
			bool leader;
			int16_t y;
			Text name, job, level;
		};

		// The online or offline part of the party list.
		struct Section
		{
			Texture header;
			Text count;
			int16_t top = 0;
			int16_t bottom = 0;
			std::vector<Line> lines;
		};

		// The members shown, minus their HP, to rebuild only on changes.
		struct Shown
		{
			int32_t id;
			std::string name;
			int16_t job, level;
			int32_t channel, map_id;

			bool operator==(const Shown& other) const;
		};

		// One buddy's half of a row; pos is relative to the window.
		struct Cell
		{
			int32_t id;
			Point<int16_t> pos;
			Text name, channel;
		};

		// The online or offline part of the buddy list, two buddies a row.
		struct BuddySection
		{
			Texture header;
			Text count;
			int16_t top = 0;
			int16_t bottom = 0;
			int16_t rows = 0;
			std::vector<Cell> cells;
		};

		// One guild member's row; y is in the member list's own space.
		struct GuildRow
		{
			int32_t id;
			int16_t y;
			Text name, job, level, title;
		};

		// The online or offline part of the guild's member list, a page at
		// a time; y is in the member list's own space.
		struct GuildSection
		{
			Texture header;
			Text count, page_text;
			bool expanded = true;
			int16_t page = 1;
			int16_t pages = 1;
			int16_t top = 0;
			std::vector<GuildRow> rows;
			// The page's first row, or "none" in its place, and the end line.
			int16_t first = 0;
			int16_t end = 0;
		};

		std::vector<Shown> currently_shown() const;
		void rebuild();
		// Lays out one section from y and returns where the next may start.
		int16_t lay_out(Section& section, const std::vector<const PartyMember*>& members, int16_t y);
		void draw_section(const Section& section) const;
		void enable(uint16_t buttonid, bool enabled);
		void update_buttons();
		const PartyMember* selected_member() const;

		void rebuild_buddies();
		void draw_buddies(const BuddySection& section, const Texture& section_row, bool legend) const;
		const Buddy* selected_buddy() const;

		void rebuild_guild();
		// Lays out one guild section from y and returns where the next may
		// start.
		int16_t lay_out(GuildSection& section, const std::vector<const GuildMember*>& members, int16_t y);
		void draw_guild() const;
		// Moves the guild sections' buttons to their headers, or hides them
		// out of view.
		void place_section_buttons();
		void scroll_guild(int16_t steps);
		// The member picked on the page shown, but never the player.
		const GuildMember* picked_member() const;
		// The same, if online.
		const GuildMember* picked_online_member() const;

		// The rank panel (UserList/Guild/GuildGrade), which the Info button
		// slides out to the window's right.
		void open_grade();
		void close_grade();
		void rebuild_grade();
		void draw_grade() const;
		// Asks the master for rank `rank`'s new title, then checks it.
		void prompt_title(int32_t rank);
		void edit_title(int32_t rank, const std::string& text);
		void save_titles();

		uint16_t tab;
		Party& party;
		BuddyList& buddies;
		Guild& guild;
		ClassicTabs tabs;
		Texture background, guild_background;

		Texture columns, row, bottom, highlight, star;
		Text none;
		Section online, offline;
		std::vector<Shown> shown;
		int32_t shown_leader = 0;
		int32_t shown_mapid = 0;
		int32_t selected = 0;

		Texture legend, buddy_row, offline_row, buddy_bottom;
		BuddySection buddies_online, buddies_offline;
		std::vector<Buddy> shown_buddies;
		int32_t picked_buddy = 0;
		bool show_all = true;
		bool filter_clicked = false;

		Texture guild_frame, notice_box, guild_columns, guild_row, guild_end;
		Text guild_name, no_guild, notice_label, notice;
		GuildMark emblem;
		bool emblem_marked = false;
		GuildSection guild_online, guild_offline;
		GuildData shown_guild;
		int32_t guild_picked = 0;
		// Where the notice has scrolled to, and the time toward its next step.
		int16_t ticker_x = 0;
		uint16_t ticker_time = 0;
		Slider guild_slider;
		int16_t guild_scroll = 0;
		int16_t guild_height = 0;

		Texture grade_background;
		std::array<Texture, 5> grade_art;
		bool grade_shown = false;
		// The titles as the master edits them, until saved, and the rank
		// picked, 0 for none.
		std::array<std::string, 5> grade_titles;
		int32_t grade_picked = 0;
		Text grade_points;
		std::array<Text, 5> grade_ranks, grade_title_texts, grade_labels;
	};
}
