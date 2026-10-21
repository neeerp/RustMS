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

#include "../Cursor.h"
#include "../Keyboard.h"

#include "../../Graphics/Text.h"
#include "../../Util/TextLines.h"

#include <functional>
#include <vector>

namespace ms
{
	class Textfield
	{
	public:
		enum State
		{
			NORMAL,
			DISABLED,
			FOCUSED
		};

		Textfield(Text::Font font, Text::Alignment alignment, Color::Name color, Rectangle<int16_t> bounds,
				  size_t limit);

		Textfield();

		void draw(Point<int16_t> position) const;

		void update(Point<int16_t> parentpos);

		void send_key(KeyType::Id type, int32_t code, bool down);

		void add_string(const std::string &str);

		void set_state(State state);

		void change_text(const std::string &text);

		// Select the whole text, which the next key typed replaces.
		void select_all();

		void set_cryptchar(int8_t character);

		// Wrap the text at width pixels instead of keeping it on one line.
		void set_wrap(uint16_t width);

		// Take several lines wrapped at `width`, showing `rows` of them
		// `linespace` px apart and scrolling to the caret: Enter breaks the
		// line with a lone "\r", as the original's multi-line edit does.
		void set_lines(uint16_t width, size_t rows, int16_t linespace);

		// The lines there are, and the first shown.
		size_t line_count() const;
		size_t first_line() const;
		// Show the lines from `first`, as a scrollbar asks.
		void scroll_lines(size_t first);

		void set_enter_callback(std::function<void(std::string)> onreturn);

		void set_key_callback(KeyAction::Id key, std::function<void(void)> action);

		Cursor::State send_cursor(Point<int16_t> cursorpos, bool clicked);

		bool empty() const;

		State get_state() const;

		Rectangle<int16_t> get_bounds() const;

		const std::string &get_text() const;

	private:
		void modifytext(const std::string &);

		bool belowlimit() const;

		// Insert a character, or a line break, if the limits allow.
		bool insert(const std::string& unit);

		// The label's lines, when there are several.
		void layout_lines();

		// Scroll the lines shown so the caret's is among them.
		void follow_marker();

		// Whether the caret's line is shown.
		bool marker_shown() const;

		// Drop the selected text.
		void erase_selection();

		Point<int16_t> marker_offset() const;

		Text textlabel;
		Text::Font font;
		Text::Alignment alignment;
		Color::Name color;
		uint16_t wrapwidth = 0;
		size_t rows = 0;
		size_t top = 0;
		int16_t linespace = 0;
		std::vector<text_lines::Line> lines;
		std::vector<Text> linelabels;
		std::string text;
		Text marker;
		bool showmarker;
		uint16_t elapsed;
		size_t markerpos;
		Rectangle<int16_t> bounds;
		Point<int16_t> parentpos;
		size_t limit;
		int8_t crypt;
		State state;
		bool selected = false;
		Text selectedlabel;

		std::map<int32_t, std::function<void(void)>> callbacks;
		std::function<void(std::string)> onreturn;
	};
}