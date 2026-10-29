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
#include <map>
#include <set>
#include <string>
#include <vector>

#include "../UIDragElement.h"

#include "../Components/ClassicTabs.h"
#include "../Components/Slider.h"

#include "../../Character/Alliance.h"
#include "../../Character/BuddyList.h"
#include "../../Character/FriendGroups.h"
#include "../../Character/Guild.h"
#include "../../Character/GuildMark.h"
#include "../../Character/GuildSort.h"
#include "../../Character/Party.h"

#include "../../Graphics/Geometry.h"

#include "RankPanelSlide.h"

namespace ms
{
	// The v83 Maple User List & Guild window (UIWindow.img/UserList), with
	// its Friend, Party, Guild, Alliance and Blacklist tabs. The Alliance
	// tab's code is in UIUserListAlliance.cpp, the Blacklist tab's in
	// UIUserListBlacklist.cpp.
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

		// Leaving the window takes its tip away.
		void remove_cursor() override;

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
			// The Friend tab's, in v83's order (FUN_0091a065).
			BT_ADD_FRIEND,
			BT_ADD_GROUP,
			BT_GROUP_WHISPER,
			BT_SHOW_ONLINE,
			BT_SHOW_ALL,
			BT_FRIEND_MOD,
			BT_FRIEND_DELETE,
			BT_FRIEND_CHAT,
			BT_FRIEND_WHISPER,
			BT_FRIEND_MESSAGE,
			BT_FRIEND_PARTY,
			BT_FRIEND_BLOCK,
			BT_FRIEND_UNBLOCK,
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
			// The Alliance tab's, in the order of FUN_0091afa5's slots.
			BT_ALLIANCE_INVITE = BT_GUILD_SECTION + 8,
			BT_ALLIANCE_WITHDRAW,
			BT_ALLIANCE_KICK,
			BT_ALLIANCE_UP,
			BT_ALLIANCE_DOWN,
			BT_ALLIANCE_CHANGE,
			BT_ALLIANCE_CHAT,
			BT_ALLIANCE_INFO,
			BT_ALLIANCE_WHISPER,
			BT_ALLIANCE_NOTICE,
			BT_ALLIANCE_PARTY_INVITE,
			// Its rank panel's.
			BT_UNION_MODIFY,
			BT_UNION_SAVE,
			// The same four buttons for each of its five guild sections.
			BT_ALLIANCE_SECTION,
			BT_TAB0 = BT_ALLIANCE_SECTION + 20,
			// The Blacklist tab's.
			BT_BLACK_ADD = BT_TAB0 + Tab::NUM_TABS,
			BT_BLACK_DELETE
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

		// One line of the friend list: a group's header, two of its members,
		// or "none" for a group shown empty.
		struct FriendLine
		{
			std::string group;
			bool header;
			// The members, 0 where there is none.
			std::array<int32_t, 2> ids;
			// A header's title, the members' names, or "none".
			std::array<Text, 2> texts;
		};

		// One guild member's row; y is in the member list's own space.
		struct GuildRow
		{
			int32_t id;
			int16_t y;
			Text name, job, level, title;
			// For the tooltips: the job's name if it was cut, whether the
			// level is unknown, and the title.
			std::string full_job;
			bool unknown_level;
			std::string title_text;
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
			// The column its members are sorted by, if any.
			guild_sort::Sort order;
			// The page's first row, or "none" in its place, and the end line.
			int16_t first = 0;
			int16_t end = 0;
		};

		// One guild of the alliance, its online members a page at a time;
		// y is in the member list's own space.
		struct AllianceSection
		{
			bool shown = false;
			int32_t guild_id = 0;
			GuildMark mark;
			bool marked = false;
			Text name, count, page_text;
			bool expanded = true;
			int16_t page = 1;
			int16_t pages = 1;
			int16_t top = 0;
			std::vector<GuildRow> rows;
			guild_sort::Sort order;
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
		void draw_buddies() const;
		void scroll_buddies(int16_t row);
		// The line under the cursor, and which half of it.
		const FriendLine* buddy_line_at(Point<int16_t> cursor_relative, size_t& half) const;
		const Buddy* selected_buddy() const;
		const FriendGroups::Group* selected_group() const;
		void fold(const std::string& group);
		// Blocks or unblocks the buddy picked, or every member of the group
		// picked (FUN_0090aba4).
		void toggle_block();
		// The GroupMod dialog, for a new group if group is blank.
		void open_group(const std::string& group);
		// The Add Buddy dialog, its name starting as name: a buddy's, to move
		// them to another group, or blank.
		void add_friend(const std::string& name);

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

		// send_cursor for all but the panels' buttons under the window.
		Cursor::State list_cursor(bool clicked, Point<int16_t> cursorpos);

		// The rank panel (UserList/Guild/GuildGrade), which the Info button
		// slides out to the window's right from behind it (RankPanelSlide).
		void open_grade();
		// Slides it back.
		void close_grade();
		// Where it is now, from the window's left.
		Point<int16_t> grade_panel() const;
		void rebuild_grade();
		void draw_grade() const;
		// Asks the master for rank `rank`'s new title, then checks it.
		void prompt_title(int32_t rank);
		void edit_title(int32_t rank, const std::string& text);
		void save_titles();

		// Loads the tab's art and makes its buttons, for the constructor.
		void init_alliance();
		void rebuild_alliance();
		// Lays out one guild of the alliance from y and returns where the
		// next may start.
		int16_t lay_out(AllianceSection& section, const GuildData& members, int16_t y);
		void draw_alliance() const;
		void place_alliance_buttons();
		void scroll_alliance(int16_t steps);
		// The member picked in the list, if the list shows them; never the
		// player.
		const GuildMember* picked_ally() const;
		// A click on the tab or its rank panel; IDLE once handled.
		Cursor::State alliance_cursor(bool clicked, Point<int16_t> cursorpos);
		Button::State alliance_button(uint16_t buttonid);
		void update_alliance_buttons(bool party_leader);

		// The rank panel (FUN_00917388), which the Alliance tab's Info button
		// opens at the window's right, with the guild panel's art.
		void open_union();
		void close_union();
		Point<int16_t> union_panel() const;
		// Drops both rank panels at once, as the window closes.
		void drop_panels();
		// Moves the panels' buttons along with them, and takes them away
		// with the panels.
		void place_panel_buttons();
		void rebuild_union();
		void draw_union() const;
		void prompt_union_title(int32_t rank);
		void edit_union_title(int32_t rank, const std::string& text);
		void save_union_titles();

		void init_blacklist();
		// Reads the names from the settings, as the tab does when shown.
		void rebuild_blacklist();
		void draw_blacklist() const;
		Cursor::State blacklist_cursor(bool clicked, Point<int16_t> cursorpos);
		// Shows text as the window's tip, or hides it for none.
		void show_tip(const char* text);
		// The Friend tab's tip for the cursor (0x91d82f, 0x90b304).
		void friend_tip(Point<int16_t> cursor_relative);
		// The guild and alliance tabs' tooltips, or nothing.
		std::string guild_tip(Point<int16_t> cursor_relative) const;
		std::string alliance_tip(Point<int16_t> cursor_relative) const;
		static std::string row_tip(const GuildRow& row, Point<int16_t> cursor_relative, int16_t list_x);
		Button::State blacklist_button(uint16_t buttonid);
		// The Add prompt, and its checks once answered (0x919214).
		void prompt_blacklist();
		void add_to_blacklist(const std::string& name);
		// Deletes the name picked (0x919506).
		void delete_from_blacklist();
		void update_blacklist_buttons();

		uint16_t tab;
		Party& party;
		BuddyList& buddies;
		Guild& guild;
		Alliance& alliance;
		ClassicTabs tabs;
		Texture background, guild_background;

		Texture columns, row, bottom, highlight, star;
		Text none;
		Section online, offline;
		std::vector<Shown> shown;
		int32_t shown_leader = 0;
		int32_t shown_mapid = 0;
		int32_t selected = 0;

		Texture friend_background, unfolded_icon, folded_icon, same_channel_icon, blocked_icon, dot;
		Text friend_count, location;
		std::vector<Buddy> shown_buddies;
		uint8_t shown_capacity = 0;
		// Point into shown_buddies.
		std::vector<FriendGroups::Group> friend_groups;
		std::vector<FriendLine> friend_lines;
		FriendGroups::Blocks blocks;
		std::set<std::string> folded;
		// v83 picks a buddy by id or a group by name, never both.
		int32_t picked_buddy = 0;
		std::string picked_group;
		bool show_online = false;
		bool filter_clicked = false;
		Slider friend_slider;
		int16_t friend_scroll = 0;
		int32_t shown_location = -1;

		Texture guild_frame, notice_box, guild_columns, guild_sorted, guild_row, guild_end;
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
		RankPanelSlide grade_slide;
		// The titles as the master edits them, until saved, and the rank
		// picked, 0 for none.
		std::array<std::string, 5> grade_titles;
		int32_t grade_picked = 0;
		Text grade_points;
		std::array<Text, 5> grade_ranks, grade_title_texts, grade_labels;

		Texture union_header, union_columns, union_sorted, union_row, union_end, union_notice_box;
		Text alliance_name, no_alliance, union_notice_label, union_notice;
		std::array<AllianceSection, 5> alliance_sections;
		AllianceData shown_alliance;
		std::vector<GuildData> shown_allies;
		GuildData shown_own;
		// Online members in all its guilds; 0 hides the alliance's name.
		size_t alliance_online = 0;
		// The member picked, by id, which survives refreshes (0x915534).
		int32_t alliance_picked = 0;
		int16_t union_ticker_x = 0;
		uint16_t union_ticker_time = 0;
		Slider alliance_slider;
		int16_t alliance_scroll = 0;
		int16_t alliance_height = 0;

		Texture union_background;
		std::array<Texture, 3> union_art;
		RankPanelSlide union_slide;
		// The panels' buttons where the panels are out.
		std::map<uint16_t, Point<int16_t>> panel_button_home;
		std::array<std::string, 5> union_titles;
		int32_t union_picked = 0;
		std::array<Text, 5> union_ranks, union_title_texts, union_labels;

		static constexpr size_t NO_BLACK_PICK = SIZE_MAX;

		Texture black_header, black_row, black_end;
		ColorBox black_fill;
		Text black_count;
		// Sorted, as shown.
		std::vector<std::string> black_names;
		std::vector<Text> black_texts;
		// By place in the list, which survives adding and deleting (0x87c).
		size_t black_picked = NO_BLACK_PICK;
		// Whether a tip of the window's shows (the Friend tab's, the Blacklist's count).
		bool tip_shown = false;
	};
}
