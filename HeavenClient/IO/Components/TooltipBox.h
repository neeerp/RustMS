#pragma once

#include "../../Graphics/Geometry.h"
#include "../../Graphics/Texture.h"

namespace ms
{
	// The layout that the v83 item, equip, and skill tooltips share, measured
	// from period footage.
	namespace tooltip
	{
		// The box's top-left against where a tooltip is drawn, which is 22
		// below the cursor.
		const Point<int16_t> OFFSET = { 20, -1 };
		// The name, after a blue dot.
		const Point<int16_t> DOT_POS = { 10, 14 };
		const Point<int16_t> NAME_POS = { 18, 7 };
		// The light box under the icon, which is drawn at twice its size.
		const Point<int16_t> ICON_POS = { 10, 31 };
		constexpr int16_t ICON_BOX = 68;
		// Lines of text are 16 apart, 2 closer than the font's spacing.
		constexpr int16_t LINE_ADJ = 2;
		// Space under the last line.
		constexpr int16_t BOTTOM = 8;

		// Where a box of the given size goes for the given draw position:
		// offset from it, but kept on the screen.
		Point<int16_t> place(Point<int16_t> position, Point<int16_t> size);
	}

	// The light box with an icon at twice its size, and the blue dot before
	// names.
	class TooltipIcon
	{
	public:
		TooltipIcon();

		void draw_dot(Point<int16_t> position) const;

		// Draws the box with the given top-left and the icon in it.
		void draw(const Texture &icon, Point<int16_t> position) const;

	private:
		ColorBox box;
		Texture dot;
	};

	// The box behind v83 tooltips, which the client drew itself: a
	// translucent navy fill inside a light line and a dark one, from footage.
	class TooltipBox
	{
	public:
		TooltipBox();

		// Draws a box whose outer edge has the given top-left and size.
		void draw(Point<int16_t> position, Point<int16_t> size) const;

	private:
		ColorBox outer;
		ColorBox inner;
		ColorBox fill;
	};
}
