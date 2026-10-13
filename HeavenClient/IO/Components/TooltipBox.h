#pragma once

#include "../../Graphics/Geometry.h"

namespace ms
{
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
