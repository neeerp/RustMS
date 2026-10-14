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

#include <string>
#include <vector>

#include "../UIElement.h"

#include "../Components/Textfield.h"

#include "../../Character/Look/CharLook.h"
#include "../../Graphics/Text.h"

namespace ms
{
	// The v83 character creation, steps 4 to 6 of the login scene. The new
	// character stands in its race's scene while a board on the right asks
	// for a name and then for the look. Each race has its own boards in
	// Login.img: NewCharKnight, NewChar, and NewCharAran.
	class UICharCreation : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::CLASSCREATION;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = false;

		// In the race select's order: Cygnus Knights, Explorers, and Aran.
		enum Race : uint8_t
		{
			CYGNUS,
			EXPLORER,
			ARAN,
			RACE_COUNT
		};

		UICharCreation(uint8_t race);

		void draw(float inter) const override;
		void update() override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;
		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

		// The server's answer to the name check.
		void send_naming_result(bool nameused);

		// The server added the new character: back to the character select.
		void send_created();

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		// The rows of the Character Setting board, top to bottom, in the
		// order of the lists in Etc.img/MakeCharInfo.
		enum Row : uint8_t
		{
			FACE,
			HAIR,
			HAIR_COLOR,
			SKIN,
			TOP,
			BOTTOM,
			SHOES,
			WEAPON,
			GENDER,
			ROW_COUNT
		};

		// Each row has a left and a right arrow, from BT_ARROWS on.
		enum Buttons : uint16_t
		{
			BT_OK,
			BT_CANCEL,
			BT_VIEWALLCHAR,
			BT_BACK,
			BT_ARROWS
		};

		void show_settings(bool settings);
		void check_name();
		void create_character();
		void show_raceselect();

		void step(uint8_t row, bool right);
		void reset_look();
		void update_look();
		void enable_buttons(bool enabled);

		std::string row_text(uint8_t row) const;
		std::string choice_name(uint8_t row, int32_t value) const;
		bool name_allowed(const std::string &name) const;

		uint8_t race;
		bool settings;
		bool female;

		Texture name_board;
		Texture setting_board;
		Texture rows[ROW_COUNT];
		Textfield name;
		Text values[ROW_COUNT];

		Texture world_box;
		Text world_name;
		Text channel_name;

		CharLook newchar;

		// MakeCharInfo's choices for each row but gender, by gender.
		std::vector<int32_t> choices[2][GENDER];
		uint8_t chosen[GENDER];
	};
}
