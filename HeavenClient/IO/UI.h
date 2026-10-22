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

#include <cstdint>

#include "ScreenshotRules.h"
#include "UIState.h"

#include "Components/ScrollingNotice.h"
#include "Components/Textfield.h"

#include "../Character/SkillMacros.h"
#include "QuickSlots.h"

namespace ms
{
	namespace storage_contents
	{
		struct Stored;
	}

	class UI : public Singleton<UI>
	{
	public:
		enum State
		{
			LOGIN,
			GAME,
			CASHSHOP
		};

		UI();

		void init();

		void draw(float alpha) const;

		void update();

		void enable();

		void disable();

		void change_state(State state);

		void quit();

		bool not_quitted() const;

		// Whether a window that holds the cursor is open.
		bool modal_open() const;

		// Whether one of v83's dialogs is open (0xbe7910, set by the dialog base 0x4eddb3): a trade or personal
		// shop, an NPC shop, storage, Duey, the item megaphone's window, the Item Guard's, Karma's, the hammer's,
		// Vega's or the kite's (reports c013, c015).
		bool dialog_open();

		void send_cursor(Point<int16_t> pos);

		void send_cursor(bool pressed);

		void send_cursor(Point<int16_t> cursorpos, Cursor::State cursorstate);

		// Whether the left button is held down.
		bool is_cursor_pressed() const;

		void send_focus(int focused);

		void send_scroll(double yoffset);

		void send_close();

		void rightclick();

		void doubleclick();

		void send_key(int32_t keycode, bool pressed);

		void set_scrollnotice(const std::string &notice);

		void focus_textfield(Textfield *textfield);

		void remove_textfield();

		void drag_icon(Icon *icon);

		void cancel_drag();

		void add_keymapping(uint8_t no, uint8_t type, int32_t action);

		void clear_tooltip(Tooltip::Parent parent);

		void show_equip(Tooltip::Parent parent, int16_t slot);

		// A real item, rather than a kind of item, also has its expiry, flags and `owner` (who made it), and is
		// `owned`. For a pet, `expiration` is when its Water of Life dries up and `remain_life` the seconds one with
		// limitedLife has left.
		void show_item(Tooltip::Parent parent, int32_t item_id, int64_t expiration = 0, int16_t flags = 0,
					   bool owned = false, int32_t remain_life = 0, const std::string &owner = {});

		// An item as storage, a trade, a personal shop or a parcel carries it.
		void show_item(Tooltip::Parent parent, const storage_contents::Stored &item);

		void
		show_skill(Tooltip::Parent parent, int32_t skill_id, int32_t level, int32_t masterlevel, int64_t expiration,
				   bool levels = true);

		void show_text(Tooltip::Parent parent, std::string text);

		// The skill macro at `index`.
		void show_macro(Tooltip::Parent parent, size_t index);

		void show_map(Tooltip::Parent parent, std::string name, std::string description);

		void show_zone(Tooltip::Parent parent, std::string title, std::string description);

		void show_line(Tooltip::Parent parent, std::string text);

		void show_help(UIElement::Type owner, std::string title, std::string description,
					   std::optional<Point<int16_t>> anchor = std::nullopt, bool above = false);

		void clear_help(UIElement::Type owner);

		Keyboard &get_keyboard();

		// The skill macros, kept across characters as v83 does (report 206).
		SkillMacros &get_skill_macros();

		// The quick slot keys, kept across characters as v83 does (report 240).
		QuickSlots &get_quick_slots();

		int64_t get_uptime();

		uint16_t get_uplevel();

		int64_t get_upexp();

		template<class T, typename...Args>
		Optional<T> emplace(Args &&...args);

		template<class T>
		Optional<T> get_element();

		void remove(UIElement::Type type);

		// Brings an open window to the front of its layer.
		void raise(UIElement::Type type);

	private:
		std::unique_ptr<UIState> state;
		Keyboard keyboard;
		SkillMacros skill_macros;
		QuickSlots quick_slots;
		Cursor cursor;
		ScrollingNotice scrollingnotice;

		Optional<Textfield> focusedtextfield;
		std::unordered_map<int32_t, bool> is_key_down;

		bool enabled;
		bool quitted;
		bool caps_lock_enabled = false;
		Screenshot::Throttle screenshots;
	};

	template<class T, typename...Args>
	Optional<T> UI::emplace(Args &&...args)
	{
		if (auto iter = state->pre_add(T::TYPE, T::TOGGLED, T::FOCUSED))
		{
			(*iter).second = std::make_unique<T>(
					std::forward<Args>(args)...
			);
		}

		return state->get(T::TYPE);
	}

	template<class T>
	Optional<T> UI::get_element()
	{
		UIElement::Type type = T::TYPE;
		UIElement *element = state->get(type);

		return static_cast<T *>(element);
	}
}