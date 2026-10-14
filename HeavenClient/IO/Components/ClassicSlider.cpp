#include "ClassicSlider.h"

#include "SliderSteps.h"

#include "../../Util/NxFiles.h"

#include <algorithm>

namespace ms
{
	ClassicSlider::ClassicSlider(Point<int16_t> pos, int16_t w, int16_t s, std::function<void(int16_t)> om)
			: onmoved(om), position(pos), width(w), steps(s)
	{
		nl::node Slider = NxFiles::ui_classic["Basic.img"]["Slider"];

		left = Slider["left"];
		mid = Slider["mid"];
		right = Slider["right"];
		thumb_normal = Slider["thumbNormal"];
		thumb_mouseover = Slider["thumbMouseOver"];
		thumb_pressed = Slider["thumbPressed"];
		thumb_disabled = Slider["thumbDisabled"];
	}

	ClassicSlider::ClassicSlider() = default;

	void ClassicSlider::draw(Point<int16_t> parentpos) const
	{
		Point<int16_t> pos = parentpos + position;
		int16_t inner = width - left.width() - right.width();

		left.draw(pos);
		mid.draw(DrawArgument(pos + Point<int16_t>(left.width(), 0), Point<int16_t>(inner, 0)));
		right.draw(pos + Point<int16_t>(width - right.width(), 0));

		Point<int16_t> thumbpos = pos + Point<int16_t>(thumb_left(), 0);

		if (!enabled)
			thumb_disabled.draw(thumbpos);
		else if (thumb == Thumb::PRESSED)
			thumb_pressed.draw(thumbpos);
		else if (thumb == Thumb::MOUSEOVER)
			thumb_mouseover.draw(thumbpos);
		else
			thumb_normal.draw(thumbpos);
	}

	Cursor::State ClassicSlider::send_cursor(Point<int16_t> cursorpos, bool clicked)
	{
		if (!enabled)
		{
			remove_cursor();

			return Cursor::State::IDLE;
		}

		if (dragging)
		{
			if (clicked)
			{
				move_thumb(cursorpos.x() - grab);

				return Cursor::State::HSCROLLIDLE;
			}

			dragging = false;
		}

		Point<int16_t> relative = cursorpos - position;

		if (relative.x() < 0 || relative.y() < 0 || relative.x() >= width || relative.y() >= HEIGHT)
		{
			thumb = Thumb::NORMAL;

			return Cursor::State::IDLE;
		}

		int16_t thumbx = relative.x() - thumb_left();
		bool on_thumb = thumbx >= 0 && thumbx < thumb_normal.width();

		if (clicked)
		{
			// A click on the track takes the thumb by its middle.
			grab = on_thumb ? thumbx : thumb_normal.width() / 2;
			dragging = true;
			thumb = Thumb::PRESSED;
			move_thumb(cursorpos.x() - grab);

			return Cursor::State::HSCROLLIDLE;
		}

		thumb = on_thumb ? Thumb::MOUSEOVER : Thumb::NORMAL;

		return Cursor::State::HSCROLL;
	}

	void ClassicSlider::remove_cursor()
	{
		dragging = false;
		thumb = Thumb::NORMAL;
	}

	void ClassicSlider::set_step(int16_t s)
	{
		step = std::clamp<int16_t>(s, 0, steps);
	}

	int16_t ClassicSlider::get_step() const
	{
		return step;
	}

	void ClassicSlider::set_enabled(bool en)
	{
		enabled = en;

		if (!enabled)
			remove_cursor();
	}

	int16_t ClassicSlider::travel() const
	{
		return width - thumb_normal.width();
	}

	int16_t ClassicSlider::thumb_left() const
	{
		return slider_steps::thumb_x(step, steps, travel());
	}

	void ClassicSlider::move_thumb(int16_t x)
	{
		int16_t next = slider_steps::nearest(x - position.x(), steps, travel());

		if (next == step)
			return;

		step = next;

		if (onmoved)
			onmoved(step);
	}
}
