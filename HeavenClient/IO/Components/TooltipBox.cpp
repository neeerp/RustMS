#include "TooltipBox.h"

#include "../Constants.h"

#include "../../Util/NxFiles.h"

#include <algorithm>

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

	Point<int16_t> tooltip::place(Point<int16_t> position, Point<int16_t> size)
	{
		position += OFFSET;

		int16_t max_width = Constants::Constants::get().get_viewwidth();
		int16_t max_height = Constants::Constants::get().get_viewheight();

		return Point<int16_t>(std::max<int16_t>(0, std::min<int16_t>(position.x(), max_width - size.x())),
							  std::max<int16_t>(0, std::min<int16_t>(position.y(), max_height - size.y())));
	}

	TooltipIcon::TooltipIcon() :
		box(tooltip::ICON_BOX, tooltip::ICON_BOX, Color::Name::WHITE, 0.65f),
		dot(NxFiles::ui_classic["UIWindow.img"]["ToolTip"]["Equip"]["Dot"]["0"])
	{}

	void TooltipIcon::draw_dot(Point<int16_t> position) const
	{
		dot.draw(position);
	}

	void TooltipIcon::draw(const Texture &icon, Point<int16_t> position) const
	{
		box.draw(DrawArgument(position));
		icon.draw(DrawArgument(position + Point<int16_t>(2, tooltip::ICON_BOX - 2), 2.0f, 2.0f));
	}
}
