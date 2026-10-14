#pragma once

#include <cstdint>
#include <functional>

#include "../Cursor.h"

#include "../../Graphics/Texture.h"

namespace ms
{
	// The v83 horizontal slider, Basic.img/Slider: a thin track with a thumb
	// that stops at one of steps + 1 places. Drag the thumb, or click the
	// track to move the thumb there. A disabled slider shows a grey thumb
	// and ignores the cursor.
	class ClassicSlider
	{
	public:
		ClassicSlider(Point<int16_t> position, int16_t width, int16_t steps, std::function<void(int16_t step)> onmoved);
		ClassicSlider();

		void draw(Point<int16_t> parentpos) const;

		// The cursor is relative to the parent. Returns IDLE when the cursor is
		// neither over the slider nor dragging it.
		Cursor::State send_cursor(Point<int16_t> cursorpos, bool clicked);

		void remove_cursor();

		// Moves the thumb without calling onmoved.
		void set_step(int16_t step);
		int16_t get_step() const;

		void set_enabled(bool enabled);

		static constexpr int16_t HEIGHT = 14;

	private:
		enum class Thumb
		{
			NORMAL,
			MOUSEOVER,
			PRESSED
		};

		int16_t travel() const;
		int16_t thumb_left() const;
		void move_thumb(int16_t x);

		std::function<void(int16_t step)> onmoved;

		Point<int16_t> position;
		int16_t width = 0;
		int16_t steps = 0;
		int16_t step = 0;
		// Where on the thumb it was grabbed.
		int16_t grab = 0;
		bool dragging = false;
		bool enabled = true;
		Thumb thumb = Thumb::NORMAL;

		Texture left;
		Texture mid;
		Texture right;
		Texture thumb_normal;
		Texture thumb_mouseover;
		Texture thumb_pressed;
		Texture thumb_disabled;
	};
}
