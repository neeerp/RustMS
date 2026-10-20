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

#include "UIPetExceptions.h"

#include "../Components/Slider.h"

#include "../../Character/Look/CharLook.h"
#include "../../Character/Look/PetLook.h"
#include "../../Graphics/Text.h"

#include <cstdint>
#include <memory>
#include <string>
#include <vector>

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
		void update() override;

		Button::State button_pressed(uint16_t buttonid) override;

		bool is_in_range(Point<int16_t> cursorpos) const override;
		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;
		void remove_cursor() override;
		void send_scroll(double yoffset) override;
		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

		void update_stats(int32_t character_id, int16_t job_id, int8_t level, int16_t fame, std::string guild, std::string alliance);

		// The mount's state, from CHAR_INFO.
		struct Mount
		{
			int32_t level;
			int32_t exp;
			int32_t tiredness;
		};

		// Show the mount the character wears, and open the taming box to it (FUN_00903db7).
		void update_mount(int32_t character_id, const Mount &mount);

		// A pet out, from CHAR_INFO.
		struct Pet
		{
			int32_t itemid;
			std::string name;
			uint8_t level;
			int16_t closeness;
			uint8_t fullness;
			// The clothes it wears, or 0.
			int32_t wear;
		};

		// Show the pets out, and open the pet box to the first (FUN_00903ce7).
		void update_pets(int32_t character_id, const std::vector<Pet> &pets);
		// Show the fame the player just raised or dropped (FAME_RESPONSE).
		void change_fame(int16_t fame);

		// Whether the exception window is open, and add to or remove from
		// its pet's list, for the item search.
		bool has_exceptions() const;
		void add_exception(int32_t itemid);
		void remove_exception(int32_t itemid);

	private:
		void give_fame(bool up) const;

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
			BT_PANEL_HIDE,
			BT_PET1,
			BT_PET2,
			BT_PET3
		};

		// What the pet box shows of a pet.
		struct PetPage
		{
			PetLook look;
			Text name;
			Text type;
			Text level;
			Text closeness;
			Text fullness;
			// Its clothes and the equipment its owner wears for it.
			std::vector<int32_t> items;
		};

		// An equipment row in the pet box.
		struct ItemRow
		{
			Texture icon;
			Text name;
			// The level it needs, for equipment.
			bool equip;
			Text level;
		};

		void set_mode(Mode mode);
		// Show the pet at `index` in the pet box (0x8ff5b0).
		void select_pet(size_t index);
		static ItemRow item_row(int32_t itemid);
		// The rows from `offset`, in the pet box or a little higher in the taming box.
		void draw_items(const std::vector<ItemRow> &rows, int16_t offset, bool taming) const;
		void set_panel(Panel panel);

		// Whether the window can show the mount: not when the saddle worn doesn't fit it (FUN_00903e24).
		bool shows_mount() const;

		// Open or close the exception window (FUN_009009bf).
		void toggle_exceptions();
		void close_exceptions();
		// Where it sits: beside the window, by the pet box when one shows (FUN_008ff7fa).
		Point<int16_t> exceptions_position() const;
		// Whether the cursor is on it, and not on the window above it.
		bool on_exceptions(Point<int16_t> cursorpos) const;
		// Whether it can open: for the player's own first pet wearing an
		// Item Ignore Pendant (FUN_00904fad).
		bool can_show_exceptions() const;
		void close();

		int32_t cid;
		bool known;
		CharLook look;
		// The character on the mount they wear, drawn in the taming box when the saddle fits it.
		bool riding;
		CharLook rider;
		DrawArgument rider_args;

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
		Text mount_name;
		Text mount_type;
		Text mount_level;
		Text mount_exp;
		Text mount_tiredness;

		std::vector<PetPage> pet_pages;
		size_t selected = 0;
		// The selected pet's equipment, from the row at the top of the list.
		std::vector<ItemRow> item_rows;
		int16_t item_offset = 0;
		Slider item_slider;
		// The saddle, in the taming box (FUN_00904c0d). It never scrolls.
		std::vector<ItemRow> taming_rows;
		Slider taming_slider;
		Texture crown;
		Texture req_level;
		std::unique_ptr<UIPetExceptions> exceptions;
		Point<int16_t> cursor;
	};
}
