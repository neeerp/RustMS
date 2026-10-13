#include "TooltipBox.h"

namespace ms
{
	namespace
	{
		constexpr float OPACITY = 0.7f;

		// Draws a one-pixel outline just inside the given rectangle.
		void outline(const ColorBox& line, Point<int16_t> position, Point<int16_t> size)
		{
			int16_t w = size.x(), h = size.y();

			line.draw(DrawArgument(position, Point<int16_t>(w, 1)));
			line.draw(DrawArgument(position + Point<int16_t>(0, h - 1), Point<int16_t>(w, 1)));
			line.draw(DrawArgument(position + Point<int16_t>(0, 1), Point<int16_t>(1, h - 2)));
			line.draw(DrawArgument(position + Point<int16_t>(w - 1, 1), Point<int16_t>(1, h - 2)));
		}
	}

	TooltipBox::TooltipBox() :
		outer(1, 1, Color::Name::BLACK, OPACITY),
		inner(1, 1, Color::Name::BLUEBELL, OPACITY),
		fill(1, 1, Color::Name::STRATOS, OPACITY)
	{}

	void TooltipBox::draw(Point<int16_t> position, Point<int16_t> size) const
	{
		if (size.x() < 4 || size.y() < 4)
			return;

		outline(outer, position, size);
		outline(inner, position + Point<int16_t>(1, 1), size - Point<int16_t>(2, 2));
		fill.draw(DrawArgument(position + Point<int16_t>(2, 2), size - Point<int16_t>(4, 4)));
	}
}
