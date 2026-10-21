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
#include "Textfield.h"

#include "../UI.h"

#include "../../Graphics/GraphicsGL.h"

#include <sstream>

namespace ms
{
	Textfield::Textfield()
	{
		text = "";
	}

	Textfield::Textfield(Text::Font font, Text::Alignment alignment, Color::Name color, Rectangle<int16_t> bounds, size_t limit) : font(font), alignment(alignment), color(color), bounds(bounds), limit(limit)
	{
		textlabel = Text(font, alignment, color, "", 0, false);
		marker = Text(font, alignment, color, "|");

		text = "";
		markerpos = 0;
		crypt = 0;
		state = State::NORMAL;
	}

	void Textfield::draw(Point<int16_t> posiiton) const
	{
		if (state == State::DISABLED)
			return;

		Point<int16_t> absp = bounds.get_left_top() + posiiton;

		if (selected && state == State::FOCUSED)
		{
			// A focused edit's selection is white on navy, as tall as the
			// font, from the top of the text (FUN_004cb1ea).
			int16_t top = absp.y() + marker.height() - 11;

			GraphicsGL::get().drawrectangle(absp.x(), top, textlabel.width(), 12, 0.0f, 0.0f, 0x7F / 255.0f, 1.0f);
			selectedlabel.draw(absp);
		}
		else if (rows > 0)
		{
			for (size_t i = top; i < linelabels.size() && i < top + rows; i++)
				linelabels[i].draw(absp + Point<int16_t>(0, linespace * static_cast<int16_t>(i - top)));
		}
		else if (text.size() > 0)
		{
			textlabel.draw(absp);
		}

		if (state == State::FOCUSED && showmarker && marker_shown())
		{
			Point<int16_t> mpos = absp + marker_offset() + Point<int16_t>(0, -1);

			marker.draw(mpos);
		}
	}

	void Textfield::update(Point<int16_t> parent)
	{
		if (state == State::DISABLED)
			return;

		parentpos = parent;
		elapsed += Constants::TIMESTEP;

		if (elapsed > 256)
		{
			showmarker = !showmarker;
			elapsed = 0;
		}
	}

	void Textfield::set_state(State st)
	{
		if (state != st)
		{
			state = st;

			if (state != State::DISABLED)
			{
				elapsed = 0;
				showmarker = true;
			}
			else
			{
				// Losing the focus sets a focused field back to NORMAL.
				UI::get().remove_textfield();
				state = State::DISABLED;
			}

			if (state == State::FOCUSED)
				UI::get().focus_textfield(this);
		}
	}

	void Textfield::set_enter_callback(std::function<void(std::string)> onr)
	{
		onreturn = onr;
	}

	void Textfield::set_key_callback(KeyAction::Id key, std::function<void(void)> action)
	{
		callbacks[key] = action;
	}

	void Textfield::send_key(KeyType::Id type, int32_t key, bool pressed)
	{
		if (pressed)
		{
			if (type == KeyType::Id::ACTION)
			{
				switch (key)
				{
				case KeyAction::Id::LEFT:
					selected = false;
					markerpos = text_lines::step_left(text, markerpos);
					follow_marker();

					break;
				case KeyAction::Id::RIGHT:
					selected = false;
					markerpos = text_lines::step_right(text, markerpos);
					follow_marker();

					break;
				case KeyAction::Id::BACK:
					if (selected)
						erase_selection();
					else if (text.size() > 0 && markerpos > 0)
					{
						size_t from = text_lines::step_left(text, markerpos);
						text.erase(from, markerpos - from);

						markerpos = from;

						modifytext(text);
					}

					break;
				case KeyAction::Id::RETURN:
					if (rows > 0)
					{
						if (selected)
							erase_selection();

						insert("\r");
					}
					else if (onreturn)
					{
						onreturn(text);
					}

					break;
				case KeyAction::Id::SPACE:
					add_string(" ");
					break;
				case KeyAction::Id::HOME:
					selected = false;
					markerpos = 0;
					follow_marker();
					break;
				case KeyAction::Id::END:
					selected = false;
					markerpos = text.size();
					follow_marker();
					break;
				case KeyAction::Id::DELETE:
					if (selected)
						erase_selection();
					else if (text.size() > 0 && markerpos < text.size())
					{
						text.erase(markerpos, text_lines::step_right(text, markerpos) - markerpos);

						modifytext(text);
					}

					break;
				default:
					if (callbacks.count(key))
						callbacks.at(key)();

					break;
				}
			}
			else if (type == KeyType::Id::TEXT)
			{
				std::stringstream ss;
				char a = static_cast<int8_t>(key);

				ss << a;

				add_string(ss.str());
			}
		}
	}

	void Textfield::add_string(const std::string& str)
	{
		if (selected && !str.empty())
			erase_selection();

		char last = 0;

		for (char c : str)
		{
			if (rows == 0)
			{
				if (belowlimit())
				{
					text.insert(markerpos, 1, c);

					markerpos++;

					modifytext(text);
				}
			}
			else if (c == '\r' || c == '\n')
			{
				// A pasted "\r\n" or "\n" breaks the line once, as Enter does.
				if (c == '\r' || last != '\r')
					insert("\r");
			}
			else
			{
				insert(std::string(1, c));
			}

			last = c;
		}
	}

	bool Textfield::insert(const std::string& unit)
	{
		if (limit > 0 ? text.size() + unit.size() > limit : !belowlimit())
			return false;

		text.insert(markerpos, unit);
		markerpos += unit.size();

		modifytext(text);

		return true;
	}

	void Textfield::modifytext(const std::string& t)
	{
		if (crypt > 0)
		{
			std::string crypted;
			crypted.insert(0, t.size(), crypt);

			textlabel.change_text(crypted);
		}
		else
		{
			textlabel.change_text(t);
		}

		text = t;

		if (rows > 0)
			layout_lines();
	}

	void Textfield::layout_lines()
	{
		auto width = [&](const std::string& line)
		{
			return Text(font, alignment, color, line, 0, false).width();
		};

		lines = text_lines::layout(text, wrapwidth, width);
		linelabels.clear();

		for (const text_lines::Line& line : lines)
			linelabels.emplace_back(font, alignment, color, text.substr(line.start, line.length), 0, false);

		follow_marker();
	}

	void Textfield::follow_marker()
	{
		if (rows == 0)
			return;

		top = text_lines::scroll_to(top, rows, lines.size(), text_lines::line_of(lines, std::min(markerpos, text.size())));
	}

	size_t Textfield::line_count() const
	{
		return lines.size();
	}

	size_t Textfield::first_line() const
	{
		return top;
	}

	void Textfield::scroll_lines(size_t first)
	{
		top = first + rows < lines.size() ? first : (lines.size() > rows ? lines.size() - rows : 0);
	}

	bool Textfield::marker_shown() const
	{
		if (rows == 0)
			return true;

		size_t line = text_lines::line_of(lines, markerpos);

		return line >= top && line < top + rows;
	}

	Cursor::State Textfield::send_cursor(Point<int16_t> cursorpos, bool clicked)
	{
		if (state == State::DISABLED)
			return Cursor::State::IDLE;

		if (get_bounds().contains(cursorpos))
		{
			if (clicked)
			{
				if (state == State::NORMAL)
					set_state(State::FOCUSED);

				// A click puts the caret down, ending a selection (FUN_004cbe89).
				selected = false;

				return Cursor::State::CLICKING;
			}
			else
			{
				return Cursor::State::CANCLICK;
			}
		}
		else
		{
			if (clicked && state == State::FOCUSED)
				set_state(State::NORMAL);

			return Cursor::State::IDLE;
		}
	}

	void Textfield::change_text(const std::string& t)
	{
		modifytext(t);

		markerpos = text.size();
		selected = false;
		follow_marker();
	}

	void Textfield::select_all()
	{
		markerpos = text.size();
		selected = !text.empty();
		selectedlabel = Text(font, alignment, Color::Name::WHITE, textlabel.get_text(), 0, false);
	}

	void Textfield::erase_selection()
	{
		selected = false;
		markerpos = 0;
		modifytext("");
	}

	void Textfield::set_cryptchar(int8_t character)
	{
		crypt = character;
	}

	void Textfield::set_wrap(uint16_t width)
	{
		wrapwidth = width;
		textlabel = Text(font, alignment, color, textlabel.get_text(), wrapwidth, false);
	}

	void Textfield::set_lines(uint16_t width, size_t shown, int16_t space)
	{
		wrapwidth = width;
		rows = shown;
		linespace = space;

		layout_lines();
	}

	Point<int16_t> Textfield::marker_offset() const
	{
		if (rows > 0)
		{
			size_t line = text_lines::line_of(lines, markerpos);

			return Point<int16_t>(linelabels[line].advance(markerpos - lines[line].start),
								  linespace * static_cast<int16_t>(line - top));
		}

		if (wrapwidth == 0)
			return Point<int16_t>(textlabel.advance(markerpos), 0);

		// Where the text before the marker ends once wrapped.
		Text before(font, alignment, color, textlabel.get_text().substr(0, markerpos), wrapwidth, false);

		return before.endoffset();
	}

	bool Textfield::belowlimit() const
	{
		if (limit > 0)
		{
			return text.size() < limit;
		}
		else
		{
			uint16_t advance = textlabel.advance(text.size());

			return (advance + 50) < bounds.get_horizontal().length();
		}
	}

	const std::string& Textfield::get_text() const
	{
		return text;
	}

	bool Textfield::empty() const
	{
		return text.empty();
	}

	Textfield::State Textfield::get_state() const
	{
		return state;
	}

	Rectangle<int16_t> Textfield::get_bounds() const
	{
		return Rectangle<int16_t>(
			bounds.get_left_top() + parentpos,
			bounds.get_right_bottom() + parentpos
			);
	}
}