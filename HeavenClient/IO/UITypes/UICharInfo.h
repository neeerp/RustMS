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

#include "../UIDragElement.h"

#include "../../Character/Look/CharLook.h"
#include "../../Graphics/Text.h"

#include <cstdint>
#include <string>

namespace ms
{
	// The v83 character info window, opened by double-clicking a character.
	class UICharInfo : public UIDragElement<PosCHARINFO>
	{
	public:
		static constexpr Type TYPE = UIElement::Type::CHARINFO;
		static constexpr bool FOCUSED = false;
		// Opening it for another character replaces the window.
		static constexpr bool TOGGLED = false;

		UICharInfo(int32_t cid);

		void draw(float inter) const override;

		Button::State button_pressed(uint16_t buttonid) override;

		bool is_in_range(Point<int16_t> cursorpos) const override;
		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

		void update_stats(int32_t character_id, int16_t job_id, int8_t level, int16_t fame, std::string guild, std::string alliance);

	private:
		// The main window alone, or lengthened to show pets, the mount, or
		// the monster book collection.
		enum Mode : uint8_t
		{
			MAIN,
			PET,
			TAMING,
			COLLECTION,
			NUM_MODES
		};

		// The list that can open on the right.
		enum Panel : uint8_t
		{
			NO_PANEL,
			ITEMS,
			WISHES
		};

		enum Buttons : uint16_t
		{
			BT_CLOSE,
			BT_FAMILY,
			BT_PARTY,
			BT_TRADE,
			BT_ITEM,
			BT_WISH,
			BT_FAME_UP,
			BT_FAME_DOWN,
			BT_PET_SHOW,
			BT_PET_HIDE,
			BT_TAMING_SHOW,
			BT_TAMING_HIDE,
			BT_COLLECTION_SHOW,
			BT_COLLECTION_HIDE,
			BT_EXCEPTION,
			BT_PANEL_HIDE
		};

		void set_mode(Mode mode);
		void set_panel(Panel panel);

		int32_t cid;
		bool known;
		CharLook look;

		Mode mode;
		Panel panel;
		Texture backgrounds[NUM_MODES];
		Texture item_list;
		Texture wish_list;

		Text name;
		Text level;
		Text job;
		Text fame;
		Text guild;
		Text alliance;
	};
}
