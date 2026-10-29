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
#include "Slider.h"

#include "SliderSteps.h"

#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

namespace ms
{
	Slider::Slider(int32_t t, Range<int16_t> ver, int16_t xp, int16_t ur, int16_t rm, std::function<void(bool)> om)
			: type(t), vertical(ver), x(xp), onmoved(om)
	{
		start = Point<int16_t>(x, vertical.first());
		end = Point<int16_t>(x, vertical.second());

		nl::node src;

		if (type == Type::CLASSIC)
		{
			src = NxFiles::ui_classic["Basic.img"]["VScr4"];
		} else
		{
			std::string VScr = "VScr";

			if (type != Type::LINE_CYAN)
				VScr += std::to_string(type);

			// UIClassic.nx stops at VScr8; later types keep the modern art.
			src = NxFiles::ui_classic["Basic.img"][VScr];

			if (!src)
				src = nl::nx::ui["Basic.img"][VScr];
		}

		nl::node dsrc = src["disabled"];

		dbase = dsrc["base"];

		dprev = dsrc["prev"];
		dnext = dsrc["next"];

		nl::node esrc = src["enabled"];

		base = esrc["base"];

		prev = TwoSpriteButton(esrc["prev0"], esrc["prev1"], start);
		next = TwoSpriteButton(esrc["next0"], esrc["next1"], end);

		thumb = TwoSpriteButton(esrc["thumb0"], esrc["thumb1"]);

		buttonheight = dnext.get_dimensions().y();

		setrows(ur, rm);

		enabled = true;
		scrolling = false;
	}

	Slider::Slider() : Slider(0, {}, 0, 0, 0, {})
	{}

	Slider::~Slider()
	{
		if (hold.slider == this)
			hold.slider = nullptr;
	}

	Slider::Hold Slider::hold;

	bool Slider::isenabled() const
	{
		return enabled;
	}

	void Slider::setenabled(bool en)
	{
		enabled = en;
	}

	void Slider::setrows(int16_t nr, int16_t ur, int16_t rm)
	{
		rowmax = rm - ur;

		if (rowmax > 0)
			rowheight = (vertical.length() - buttonheight * 2) / rowmax;
		else
			rowheight = 0;

		row = nr;
	}

	void Slider::setrows(int16_t ur, int16_t rm)
	{
		setrows(0, ur, rm);
	}

	void Slider::setvertical(Range<int16_t> ver)
	{
		vertical = ver;
		start = Point<int16_t>(x, vertical.first());
		end = Point<int16_t>(x, vertical.second());
		prev.set_position(start);
		next.set_position(end);

		if (rowmax > 0)
			rowheight = (vertical.length() - buttonheight * 2) / rowmax;
		else
			rowheight = 0;
	}

	Range<int16_t> Slider::getvertical() const
	{
		return vertical;
	}

	void Slider::draw(Point<int16_t> position) const
	{
		Point<int16_t> base_pos = position + start;
		Point<int16_t> fill = Point<int16_t>(0, vertical.length() + buttonheight - 2);
		DrawArgument base_arg = DrawArgument(Point<int16_t>(base_pos.x(), base_pos.y() + 1), fill);

		int16_t height = dbase.height();
		int16_t maxheight = vertical.first() + height;

		while (maxheight < vertical.second())
		{
			dbase.draw(position + Point<int16_t>(start.x(), maxheight));

			maxheight += height;
		}

		if (enabled)
		{
			if (rowmax > 0)
			{
				prev.draw(position);
				next.draw(position);
				thumb.draw(position + getthumbpos());
			}
			else
			{
				dprev.draw(position + start);
				dnext.draw(position + end);
			}
		} else
		{
			dprev.draw(position + start);
			dnext.draw(position + end);
		}
	}

	void Slider::remove_cursor()
	{
		scrolling = false;

		if (hold.slider == this)
			hold.slider = nullptr;

		thumb.set_state(Button::State::NORMAL);
		next.set_state(Button::State::NORMAL);
		prev.set_state(Button::State::NORMAL);
	}

	Point<int16_t> Slider::getthumbpos() const
	{
		int16_t y =
				rowheight == 0 ?
				vertical.first() + buttonheight + slider_steps::thumb_x(row, rowmax, travel()) :
				row < rowmax ?
				vertical.first() + row * rowheight + buttonheight :
				vertical.second() - buttonheight * 2 - 2;

		return Point<int16_t>(x, y);
	}

	int16_t Slider::travel() const
	{
		return vertical.length() - buttonheight * 3 - 2;
	}

	Cursor::State Slider::send_cursor(Point<int16_t> cursor, bool pressed)
	{
		Point<int16_t> relative = cursor - start;

		if (scrolling)
		{
			if (pressed && rowheight == 0)
			{
				// More rows than pixels: the thumb follows the cursor, a row
				// at a time.
				int16_t target = slider_steps::nearest(relative.y() - buttonheight * 2, rowmax, travel());

				while (row < target)
				{
					row++;
					onmoved(false);
				}

				while (row > target)
				{
					row--;
					onmoved(true);
				}

				return Cursor::State::VSCROLLIDLE;
			} else if (pressed)
			{
				int16_t thumby = row * rowheight + buttonheight * 2;
				int16_t delta = relative.y() - thumby;

				if (delta > rowheight / 2 && row < rowmax)
				{
					row++;
					onmoved(false);
				} else if (delta < -rowheight / 2 && row > 0)
				{
					row--;
					onmoved(true);
				}

				return Cursor::State::VSCROLLIDLE;
			} else
			{
				scrolling = false;
			}
		} else if (hold.slider == this)
		{
			// Like the original's mouse capture: the repeat in update_held
			// follows the cursor until the button is released.
			if (pressed)
			{
				hold.cursor = cursor;

				return Cursor::State::VSCROLLIDLE;
			}

			hold.slider = nullptr;
		}

		if (relative.x() < 0 || relative.y() < 0 || relative.x() > 8 || relative.y() > vertical.length() + buttonheight)
		{
			thumb.set_state(Button::State::NORMAL);
			next.set_state(Button::State::NORMAL);
			prev.set_state(Button::State::NORMAL);

			return Cursor::State::IDLE;
		}

		Point<int16_t> thumbpos = getthumbpos();

		if (thumb.bounds(thumbpos).contains(cursor))
		{
			if (pressed)
			{
				scrolling = true;
				thumb.set_state(Button::State::PRESSED);

				return Cursor::State::VSCROLLIDLE;
			} else
			{
				thumb.set_state(Button::State::NORMAL);

				return Cursor::State::VSCROLL;
			}
		} else
		{
			thumb.set_state(Button::State::NORMAL);
		}

		if (prev.bounds(Point<int16_t>()).contains(cursor))
		{
			if (pressed)
			{
				press(Part::PREV, cursor);
				prev.set_state(Button::State::PRESSED);

				return Cursor::State::VSCROLLIDLE;
			} else
			{
				prev.set_state(Button::State::MOUSEOVER);

				return Cursor::State::VSCROLL;
			}
		} else
		{
			prev.set_state(Button::State::NORMAL);
		}

		if (next.bounds(Point<int16_t>()).contains(cursor))
		{
			if (pressed)
			{
				press(Part::NEXT, cursor);
				next.set_state(Button::State::PRESSED);

				return Cursor::State::VSCROLLIDLE;
			} else
			{
				next.set_state(Button::State::MOUSEOVER);

				return Cursor::State::VSCROLL;
			}
		} else
		{
			next.set_state(Button::State::NORMAL);
		}

		if (cursor.y() < vertical.second())
		{
			if (pressed)
			{
				press(cursor.y() < thumbpos.y() ? Part::PAGE_UP : Part::PAGE_DOWN, cursor);

				return Cursor::State::VSCROLLIDLE;
			}
		}

		return Cursor::State::VSCROLL;
	}

	Slider::Part Slider::part_at(Point<int16_t> cursor) const
	{
		Point<int16_t> relative = cursor - start;

		if (relative.x() < 0 || relative.y() < 0 || relative.x() > 8 || relative.y() > vertical.length() + buttonheight)
			return Part::NONE;

		Point<int16_t> thumbpos = getthumbpos();

		if (thumb.bounds(thumbpos).contains(cursor))
			return Part::THUMB;

		if (prev.bounds(Point<int16_t>()).contains(cursor))
			return Part::PREV;

		if (next.bounds(Point<int16_t>()).contains(cursor))
			return Part::NEXT;

		if (cursor.y() < vertical.second())
			return cursor.y() < thumbpos.y() ? Part::PAGE_UP : Part::PAGE_DOWN;

		return Part::NONE;
	}

	void Slider::press(Part part, Point<int16_t> cursor)
	{
		hold = { this, part, cursor, std::chrono::steady_clock::now() };
		step(part);
	}

	void Slider::step(Part part)
	{
		// The track pages by a sixteenth of the positions, but at least 16.
		int16_t page = std::max<int16_t>(16, (rowmax + 1) / 16);
		int16_t count = part == Part::PAGE_UP || part == Part::PAGE_DOWN ? page : 1;
		bool upwards = part == Part::PREV || part == Part::PAGE_UP;

		for (int16_t i = 0; i < count; i++)
		{
			if (upwards && row > 0)
			{
				row--;
				onmoved(true);
			} else if (!upwards && row < rowmax)
			{
				row++;
				onmoved(false);
			}
		}
	}

	void Slider::update_held(bool mousedown)
	{
		if (!hold.slider)
			return;

		if (!mousedown)
		{
			hold.slider = nullptr;
			return;
		}

		auto now = std::chrono::steady_clock::now();

		if (now - hold.last <= std::chrono::milliseconds(50))
			return;

		if (hold.slider->part_at(hold.cursor) != hold.part)
			return;

		hold.last = now;
		hold.slider->step(hold.part);
	}

	void Slider::send_scroll(double yoffset)
	{
		if (yoffset < 0 && row < rowmax)
		{
			row++;
			onmoved(false);
		}

		if (yoffset > 0 && row > 0)
		{
			row--;
			onmoved(true);
		}
	}
}