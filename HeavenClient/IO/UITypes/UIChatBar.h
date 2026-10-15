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

#include <cstddef>

#include <cstdint>
#include <optional>
#include <unordered_map>
#include <vector>

#include "../Messages.h"
#include "../UIElement.h"

#include "../Components/Textfield.h"

#include "../../Template/EnumMap.h"

namespace ms
{
	// The v83 chat: a strip in the status bar that shows the latest line or,
	// after Enter, the input field, and a log above the bar that the button
	// at the strip's right end expands. The expanded log keeps the input
	// field in the strip.
	class UIChatBar : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::CHATBAR;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = true;

		enum LineType
		{
			UNK0,
			WHITE,
			RED,
			BLUE,
			YELLOW,
			GREY,
			// Party chat and party notices
			PARTY,
			// Buddy chat
			BUDDY,
			PINK
		};

		// Who a typed line goes to
		enum class Target
		{
			ALL,
			PARTY,
			BUDDY
		};

		UIChatBar();

		void draw(float inter) const override;

		void update() override;

		void update_screen(int16_t new_width, int16_t new_height) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		bool is_in_range(Point<int16_t> cursorpos) const override;

		Cursor::State send_cursor(bool clicking, Point<int16_t> cursorpos) override;

		void send_scroll(double yoffset) override;

		void remove_cursor() override;

		UIElement::Type get_type() const override;

		void send_chatline(const std::string &line, LineType type);

		void display_message(Messages::Type line, UIChatBar::LineType type);

		// Expand or collapse the log above the status bar.
		void toggle_chat();

		void toggle_chat(bool chat_open);

		// Open or close the input field.
		void toggle_chatfield();

		void toggle_chatfield(bool chatfield_open);

		bool is_chatopen();

		bool is_chatfieldopen();

		// Send typed lines to everyone nearby, the party, or online buddies,
		// and open the input field.
		void set_target(Target new_target);
		// Open the chat field with "/w name " typed, ready for the message.
		void start_whisper(const std::string& name);

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		enum Arrow
		{
			PREV,
			NEXT
		};

		void scroll(int16_t rows);

		// The log shows the latest line in the strip, that line above the
		// strip while typing, or several lines once expanded.
		int16_t get_logrows() const;
		int16_t get_logheight() const;
		int16_t get_logtop() const;

		// The scroll bar beside the log: its top and its length.
		Point<int16_t> get_scrollbar_pos() const;
		int16_t get_scrollbar_length() const;
		// The scroll bar's positions and the current one, the first being
		// the oldest lines. Fewer than two leave it disabled.
		int16_t get_scrollrange() const;
		int16_t get_scrollpos() const;
		void set_scrollpos(int16_t pos);
		// Where the top of a thumb this tall is below the scroll bar's, if
		// it has one.
		std::optional<int16_t> get_thumb_offset(int16_t height) const;

		void send_party_chat(const std::string& message);
		void send_buddy_chat(const std::string& message);

		static constexpr int16_t CHATROWHEIGHT = 13;
		static constexpr int16_t CHATROWS = 5;
		static constexpr int16_t LOG_WIDTH = 580;
		static constexpr int16_t LOG_HEIGHT = 70;
		static constexpr int16_t LINE_LOG_HEIGHT = 24;
		static constexpr time_t MESSAGE_COOLDOWN = 1'000;

		enum Buttons : uint16_t
		{
			BT_OPENCHAT,
			BT_CLOSECHAT
		};

		bool chatopen;
		bool chatfieldopen;
		Texture backgrnd;
		Texture backgrnd2;
		Texture chattarget;
		Text chattarget_label;
		Target chat_target;
		Texture arrows[2][2];
		Texture arrows_disabled[2];
		Texture scrollbase;
		Texture scrollbase_disabled;
		Texture thumb[2];
		Textfield chatfield;

		// Whether each arrow is held down
		bool arrow_pressed[2];
		// Whether the mouse was down over the scroll bar, and where the
		// thumb was grabbed, if it was.
		bool scrollbar_pressed;
		bool thumb_dragging;
		int16_t drag_y;
		int16_t drag_pos;

		EnumMap<Messages::Type, time_t> message_cooldowns;
		std::vector<std::string> lastentered;
		size_t lastpos;

		int16_t rowpos;
		int16_t rowmax;
		std::unordered_map<int16_t, Text> rowtexts;
	};
}
