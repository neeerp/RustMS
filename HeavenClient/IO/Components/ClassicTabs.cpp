#include "ClassicTabs.h"

#include "../../Util/NxFiles.h"

namespace ms
{
	ClassicTabs::ClassicTabs(nl::node enabled_src, nl::node disabled_src, int16_t fill) : fill_width(fill)
	{
		nl::node Tab5 = NxFiles::ui_classic["Basic.img"]["Tab5"];

		for (size_t i = 0; i < 2; i++)
		{
			left[i] = Tab5["left" + std::to_string(i)];
			this->fill[i] = Tab5["fill" + std::to_string(i)];
			right[i] = Tab5["right" + std::to_string(i)];
		}

		for (size_t i = 0; i < 3; i++)
			middle[i] = Tab5["middle" + std::to_string(i)];

		for (nl::node label : enabled_src)
			enabled.emplace_back(label);

		for (nl::node label : disabled_src)
			disabled.emplace_back(label);
	}

	ClassicTabs::ClassicTabs() = default;

	void ClassicTabs::draw(Point<int16_t> position, uint16_t count, uint16_t selected) const
	{
		if (count == 0)
			return;

		Point<int16_t> pos = position;

		left[selected == 0].draw(pos);
		pos.shift_x(EDGE);

		for (uint16_t i = 0; i < count; i++)
		{
			bool is_selected = i == selected;

			fill[is_selected].draw(DrawArgument(pos, Point<int16_t>(fill_width, 0)));

			const std::vector<Texture> &labels = is_selected ? enabled : disabled;

			if (i < labels.size())
			{
				Point<int16_t> dimensions = labels[i].get_dimensions();
				int16_t x = fill_width / 2 - dimensions.x() / 2;
				int16_t y = LABEL_BOTTOM - dimensions.y();

				labels[i].draw(pos + Point<int16_t>(x, y));
			}

			pos.shift_x(fill_width);

			if (i + 1 < count)
			{
				size_t piece = is_selected ? 1 : (i + 1 == selected ? 2 : 0);

				middle[piece].draw(pos);
				pos.shift_x(2 * EDGE);
			}
		}

		right[selected + 1 == count].draw(pos);
	}

	int16_t ClassicTabs::pitch() const
	{
		return fill_width + 2 * EDGE;
	}
}
