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
#include <string>
#include <vector>

#include "../UIDragElement.h"

#include "../Components/ClassicTabs.h"

#include "../../Character/Party.h"

namespace ms
{
	// The v83 Maple User List & Guild window (UIWindow.img/UserList). Only
	// the Party tab works so far; the others show an empty window.
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

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

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
			BT_TAB0
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

		std::vector<Shown> currently_shown() const;
		void rebuild();
		// Lays out one section from y and returns where the next may start.
		int16_t lay_out(Section& section, const std::vector<const PartyMember*>& members, int16_t y);
		void draw_section(const Section& section) const;
		void enable(uint16_t buttonid, bool enabled);
		void update_buttons();
		const PartyMember* selected_member() const;

		uint16_t tab;
		Party& party;
		ClassicTabs tabs;

		Texture columns, row, bottom, highlight, star;
		Text none;
		Section online, offline;
		std::vector<Shown> shown;
		int32_t shown_leader = 0;
		int32_t shown_mapid = 0;
		int32_t selected = 0;
	};
}
