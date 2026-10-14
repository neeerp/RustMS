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

#include "../Components/Slider.h"
#include "../Components/Textfield.h"

#include "../../Character/Look/CharLook.h"
#include "../../Graphics/Animation.h"

#include <array>

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

		UIElement::Type get_type() const override;

		// Opens a new messenger, or leaves the one we are in.
		static void toggle();

		// The server seated us in a slot; shows the window.
		static void seat(uint8_t slot);

		void set_member(uint8_t slot, const CharLook &look, const std::string &name);

		void remove_member(uint8_t slot);

		// A line someone said, formatted as "name : text".
		void add_chat(const std::string &line);

		// A notice from the messenger itself.
		void add_note(const std::string &note);

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		void join(uint8_t slot);

		void clear_member(uint8_t slot);

		void leave();

		void submit(const std::string &text);

		void ask_invite();

		void invite(const std::string &name);

		void add_line(const std::string &line);

		void scroll_to_bottom();

		void set_minimized(bool minimized);

		int16_t log_rows() const;

		enum Buttons : uint16_t
		{
			MIN,
			MAX,
			CLOSE,
			ENTER,
			SLOT
		};

		struct Member
		{
			bool present = false;
			CharLook look;
			Text name;
			Animation balloon;
			uint16_t balloon_left = 0;
		};

		std::array<Member, SLOTS> members;
		uint8_t own_slot;

		Texture backgrnd;
		Texture minimized_backgrnd;
		Texture blink;
		std::array<Texture, SLOTS + 1> namebars;

		std::vector<Text> log;
		int16_t log_height;
		int16_t log_row;
		Slider slider;
		Textfield input;

		bool minimized;
		bool unread;
		uint16_t blink_elapsed;
	};
}
