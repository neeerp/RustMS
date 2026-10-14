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
	// at the strip's right end expands.
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
			PINK
		};

		// Who a typed line goes to
		enum class Target
		{
			ALL,
			PARTY
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

		// Send typed lines to everyone nearby or to the party, and open the
		// input field.
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

		int16_t get_logtop() const;

		Point<int16_t> get_arrow_pos(Arrow arrow) const;

		Rectangle<int16_t> get_arrow_bounds(Arrow arrow) const;

		void send_party_chat(const std::string& message);

		static constexpr int16_t CHATROWHEIGHT = 13;
		static constexpr int16_t CHATROWS = 5;
		static constexpr int16_t STRIP_WIDTH = 551;
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
		Texture arrows[2][3];
		Texture arrows_disabled[2];
		Texture scrollbase;
		Textfield chatfield;

		// The arrow under the cursor, and whether it is held down.
		int16_t arrow_state[2];

		EnumMap<Messages::Type, time_t> message_cooldowns;
		std::vector<std::string> lastentered;
		size_t lastpos;

		int16_t rowpos;
		int16_t rowmax;
		std::unordered_map<int16_t, Text> rowtexts;
	};
}
