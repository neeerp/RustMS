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

#include "../UIElement.h"

#include "../Components/ChatBalloon.h"

#include "../../Character/Look/CharLook.h"
#include "../../Net/Login.h"

namespace ms
{
	// The v83 character select, step 2 of the login scene. Up to three
	// characters stand on the rope bridge; picking one lights it from above
	// and unrolls a scroll of its stats over it.
	class UICharSelect : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::CHARSELECT;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = false;

		UICharSelect(std::vector<CharEntry> characters, int8_t characters_count, int32_t slots, int8_t require_pic);

		void draw(float alpha) const override;
		void update() override;

		void doubleclick(Point<int16_t> cursorpos) override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

		void add_character(CharEntry &&character);

		void post_add_character();

		void remove_character(int32_t id);

		const CharEntry &get_character(int32_t id);

		// Whether the list's 15 entries are all taken (FUN_005f7e7a).
		bool full() const;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		void update_buttons();
		void select_character(int16_t index);
		void change_page(int16_t page);
		void move_focus(int8_t step);
		void enter_game();
		void delete_character();

		// The slot on this page under the cursor, or -1.
		int16_t slot_at(Point<int16_t> cursorpos) const;

		void request_pic();
		void check_pic(const std::string entered_pic) const;

		// The most a character list holds.
		static constexpr int8_t MAX_CHARACTERS = 15;

		enum Buttons : uint16_t
		{
			BT_SELECT,
			BT_NEW,
			BT_DELETE,
			BT_VIEWALLCHAR,
			BT_BACK
		};

		enum class Scroll : uint8_t
		{
			CLOSED,
			OPENING,
			OPEN
		};

		// A page arrow on the bridge; it lights up under the cursor.
		struct PageArrow
		{
			Texture normal;
			Texture hovered;
			Point<int16_t> position;
			bool hover = false;
		};

		std::vector<CharEntry> characters;
		std::vector<CharLook> charlooks;
		std::vector<Text> names;
		int8_t characters_count;
		int32_t slots;
		int8_t require_pic;

		// Indexes into `characters`; -1 when none.
		int16_t selected = -1;
		int16_t page = 0;
		int16_t page_count = 1;

		// The menu sign the Tab key points at, or -1 while the keys move
		// between characters.
		int16_t menu_focus = -1;
		Animation focus_hand[3];

		Texture signposts[3];
		Texture nametags[2][3];

		Animation beam;
		Animation sparkle;
		bool beam_done = true;
		bool sparkle_done = true;

		Scroll scroll = Scroll::CLOSED;
		Animation scroll_open[2];
		Texture scroll_opened[2];
		Texture charinfo[2];
		Texture rank_icons[3];
		Texture job_icons[5];

		PageArrow page_left;
		PageArrow page_right;

		ChatBalloonHorizontal chatballoon;
		ChatBalloonHorizontal hintballoon;
	};
}
