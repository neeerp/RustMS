#pragma once

#include "Tooltip.h"

#include "../../Data/CashCatalog.h"
#include "../../Graphics/Text.h"
#include "../../Graphics/Texture.h"

#include <vector>

namespace ms
{
	// The Cash Shop's tooltip for a package (FUN_008efcb7, report 202b): its
	// name and description, then each content with its icon, its price
	// against what it costs on its own, and the total of both, on the plain
	// tooltip canvas.
	class PackageTooltip : public Tooltip
	{
	public:
		PackageTooltip();

		// all is every commodity in Commodity.img, on sale or not.
		void set_package(int32_t item_id, const std::vector<cash_catalog::Commodity> &all);
		void reset();

		// Draws the box with its top left at the given point, kept on the
		// screen.
		void draw(Point<int16_t> position) const override;

	private:
		struct Row
		{
			Texture icon;
			Text name;
			Text price;
			// The discounted price, struck through price, or empty.
			Text discount;
			Text period;
			int16_t y;
			int16_t name_y;
			int16_t price_y;
			int32_t percent;
		};

		void draw_badge(Point<int16_t> position, int32_t percent) const;

		int32_t item_id = 0;
		int16_t width = 0;
		int16_t height = 0;
		Text title;
		Text desc;
		std::vector<Row> rows;
		int16_t footer_y = 0;
		Text total;
		Text total_paid;
		int32_t total_percent = 0;

		Texture background;
		Texture line;
		Texture arrow;
		Texture total_label;
		Texture badge_left;
		Texture badge_right;
		std::vector<Texture> badge_digits;
	};
}
