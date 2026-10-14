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

#include "../UIElement.h"

#include "../../Graphics/Text.h"

namespace ms
{
	// The v83 race select, step 3 of the login scene: Cygnus Knights,
	// Explorers, and Aran stand under the "Choose Character Type" banner,
	// and the picked one's description sits on a board below them.
	class UIRaceSelect : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::RACESELECT;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = false;

		UIRaceSelect();

		void draw(float inter) const override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

		bool check_name(std::string name) const;

		void send_naming_result(bool nameused);

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		// In the order they stand, left to right.
		enum Race : uint8_t
		{
			CYGNUS,
			EXPLORER,
			ARAN,
			RACE_COUNT
		};

		enum Buttons : uint16_t
		{
			BT_CYGNUS,
			BT_EXPLORER,
			BT_ARAN,
			BT_SELECT,
			BT_VIEWALLCHAR,
			BT_BACK
		};

		void select_race(uint8_t race);

		void make_character();

		void show_charselect();

		std::string to_lower(std::string value) const;

		uint8_t selected;
		Texture title;
		Texture race_on[RACE_COUNT];
		Texture race_text[RACE_COUNT];
		Texture world_box;
		Text world_name;
		Text channel_name;
	};
}
