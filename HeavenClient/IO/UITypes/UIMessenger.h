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

#include "../MessengerRules.h"

#include "../Components/ChatFloodGuard.h"
#include "../Components/Slider.h"
#include "../Components/Textfield.h"

#include "../../Character/Look/CharLook.h"
#include "../../Graphics/Animation.h"

#include <array>
#include <string>
#include <vector>

namespace ms
{
	// The Maple Messenger: a chat room of up to three characters anywhere in
	// the world. The server seats us; the window lives while we are seated.
	class UIMessenger : public UIDragElement<PosMESSENGER>
	{
	public:
		static constexpr Type TYPE = UIElement::Type::MESSENGER;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = true;

		static constexpr size_t SLOTS = 3;

		UIMessenger();

		void draw(float inter) const override;

		void update() override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;

		void send_scroll(double yoffset) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		void remove_cursor() override;

		UIElement::Type get_type() const override;

		// Opens a new messenger, or folds the open one to its title and back.
		static void toggle();

		// The server seated us in a slot; shows the window. A negative slot
		// means we couldn't join, and v83 seats us first (0x84f281).
		static void seat(int8_t slot);

		// Someone took a seat, announced in the log if `announce`. `channel`
		// counts from zero.
		void set_member(uint8_t slot, const CharLook &look, const std::string &name, uint8_t channel, bool announce = false);

		// Someone left, said in the log (0x84f36c).
		void remove_member(uint8_t slot);

		// A new look for whoever sits in `slot`; an empty seat stays empty
		// (0x85194f).
		void update_look(uint8_t slot, const CharLook &look);

		// Empties a seat without a word, as mode 8 does (0x8519f2).
		void empty_seat(uint8_t slot);

		// A line from the room: something said, as "name : text", or a notice
		// that someone started or stopped typing (0x8515d7).
		void hear(const std::string &line);

		// A notice from the messenger itself.
		void add_note(const std::string &note);

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		void join(uint8_t slot);

		void clear_member(uint8_t slot);

		// The three sizes the window folds through (0x850769).
		enum class Size
		{
			FULL,
			SHORT,
			TITLE
		};

		void leave();

		void drop_focus();

		void ask_leave();

		void submit();

		void say(const std::string &text);

		void invite(const std::string &name);

		void add_line(const std::string &line, Color::Name color);

		void keep_said(const std::string &line);

		void scroll_to_bottom();

		void set_size(Size next);

		void refresh_typing();

		void show_channel(Point<int16_t> cursorpos);

		void clear_channel();

		int16_t log_rows() const;

		enum Buttons : uint16_t
		{
			MIN,
			MAX,
			CLOSE,
			ENTER,
			CLAIM
		};

		struct Member
		{
			bool present = false;
			CharLook look;
			Text name;
			Animation balloon;
			uint8_t channel = 0;
			bool typing = false;
			bool balloon_shown = false;
		};

		std::array<Member, SLOTS> members;
		uint8_t own_slot;

		Texture backgrnd;
		Texture short_backgrnd;
		Texture title_backgrnd;
		Texture blink;
		std::array<Texture, SLOTS + 1> namebars;

		std::vector<Text> log;
		int16_t log_height;
		int16_t log_row;
		Slider slider;
		Textfield input;
		Text typing_row;

		// The chat lines of the log, for the report button.
		std::vector<std::string> said;

		messenger_rules::TypingGate typing_gate;
		messenger_rules::History history;
		ChatFloodGuard flood_guard;

		Size size;
		bool unread;
		uint16_t blink_elapsed;
		bool channel_tip;
	};
}
