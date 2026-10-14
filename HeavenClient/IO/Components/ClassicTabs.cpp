#include "ClassicTabs.h"

#include "../../Util/NxFiles.h"

namespace ms
{
	// Measured from period footage of the skill window and the quest log.
	const ClassicTabs::Style ClassicTabs::TAB5 = { "Tab5", 6, 21, 17, false };
	const ClassicTabs::Style ClassicTabs::TAB2 = { "Tab2", 4, 19, 6, true };

	ClassicTabs::ClassicTabs(const Style& st, nl::node enabled_src, nl::node disabled_src, int16_t fill) : style(st), fill_width(fill)
	{
		nl::node strip = NxFiles::ui_classic["Basic.img"][style.node];

		for (size_t i = 0; i < 2; i++)
		{
			left[i] = strip["left" + std::to_string(i)];
			this->fill[i] = strip["fill" + std::to_string(i)];
			right[i] = strip["right" + std::to_string(i)];
		}

		for (size_t i = 0; i < 3; i++)
			middle[i] = strip["middle" + std::to_string(i)];

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
		pos.shift_x(style.edge);

		for (uint16_t i = 0; i < count; i++)
		{
			bool is_selected = i == selected;

			fill[is_selected].draw(DrawArgument(pos, Point<int16_t>(fill_width, 0)));

			const std::vector<Texture> &labels = is_selected ? enabled : disabled;

			if (i < labels.size())
			{
				Point<int16_t> dimensions = labels[i].get_dimensions();
				int16_t x = fill_width / 2 - dimensions.x() / 2;
				int16_t y = style.label_top ? style.label_y : style.label_y - dimensions.y();

				labels[i].draw(pos + Point<int16_t>(x, y));
			}

			pos.shift_x(fill_width);

			if (i + 1 < count)
			{
				size_t piece = is_selected ? 1 : (i + 1 == selected ? 2 : 0);

				middle[piece].draw(pos);
				pos.shift_x(2 * style.edge);
			}
		}

		right[selected + 1 == count].draw(pos);
	}

	int16_t ClassicTabs::pitch() const
	{
		return fill_width + 2 * style.edge;
	}

	int16_t ClassicTabs::height() const
	{
		return style.height;
	}
}
