#include "PackageTooltip.h"

#include "PackageTooltipLayout.h"
#include "TooltipBox.h"

#include "../Constants.h"

#include "../../Data/ItemData.h"
#include "../../Gameplay/MapleMap/MapTooltipRules.h"
#include "../../Graphics/GraphicsGL.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// v83 fills the box with 0xA0000040.
		constexpr float FILL_BLUE = 0x40 / 255.0f, FILL_ALPHA = 0xA0 / 255.0f;
		// The description wraps 35 short of the box's width.
		constexpr int16_t DESC_MARGIN = 35;
		// A mount's description, which v83 measures for the height alone,
		// wraps at 175.
		constexpr int16_t MOUNT_WRAP = 175;
		constexpr int16_t ICON_BOX = 34;
		// The art's offsets from the price's top: the strike, the arrow
		// after it, and the discounted price after the arrow.
		constexpr int16_t STRIKE_Y = 4, ARROW_GAP = 10, ARROW_Y = 1, ARROW_ADVANCE = 14;
		// The badge's offset from the box's right edge, and its digits'
		// pitch.
		constexpr int16_t BADGE_RIGHT = 40, DIGIT_PITCH = 5;

		// How far down to draw text whose top v83 puts at y: HC puts a
		// line's baseline a full line below where it's drawn.
		int16_t text_y(int16_t y)
		{
			static const int16_t line = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE, "|").height();

			return y + map_tooltips::BASELINE - line;
		}

		Text label(Color::Name color, const std::string &text, uint16_t wrap = 0)
		{
			return Text(Text::Font::A12M, Text::Alignment::LEFT, color, text, wrap, wrap > 0, tooltip::LINE_ADJ);
		}

		// How many lines a wrapped text takes, 16 apart.
		int16_t lines(const Text &text)
		{
			if (text.empty())
				return 0;

			return (text.height() + package_tooltip::LINE_HEIGHT / 2) / package_tooltip::LINE_HEIGHT;
		}
	}

	PackageTooltip::PackageTooltip()
	{
		nl::node art = NxFiles::ui_classic["CashShop.img"]["CSDiscount"];

		background = art["backgrnd"];
		line = art["Line"];
		arrow = art["arrow"];
		total_label = art["total"];
		badge_left = art["w"];
		badge_right = art["e"];

		for (int32_t digit = 0; digit < 10; digit++)
			badge_digits.emplace_back(art[std::to_string(digit)]);
	}

	void PackageTooltip::set_package(int32_t id, const std::vector<cash_catalog::Commodity> &all)
	{
		if (id == item_id)
			return;

		item_id = id;
		rows.clear();

		const ItemData &package = ItemData::get(item_id);
		title = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::PACKAGEYELLOW, package.get_name());

		// The contents, by serial number, as commodities.
		std::vector<const cash_catalog::Commodity *> contents;

		for (nl::node sn : nl::nx::etc["CashPackage.img"][std::to_string(item_id)]["SN"])
		{
			int32_t serial = static_cast<int32_t>(sn.get_integer());

			for (const cash_catalog::Commodity &commodity : all)
			{
				if (commodity.sn == serial)
				{
					contents.push_back(&commodity);
					break;
				}
			}
		}

		std::vector<int16_t> name_widths;

		for (const cash_catalog::Commodity *content : contents)
			name_widths.push_back(label(Color::Name::WHITE, ItemData::get(content->item_id).get_name()).width());

		width = package_tooltip::width(name_widths);

		const std::string &description = package.get_desc();
		desc = description.empty() ? Text() : label(Color::Name::WHITE, description, width - DESC_MARGIN);
		int16_t desc_height = package_tooltip::desc_height(lines(desc));

		// v83 makes room under the footer for the first mount's description
		// without drawing it.
		int16_t mount_height = 0;

		for (const cash_catalog::Commodity *content : contents)
		{
			if (content->item_id / 10000 == 190)
			{
				Text mount = label(Color::Name::WHITE, ItemData::get(content->item_id).get_desc(), MOUNT_WRAP);
				mount_height = package_tooltip::desc_height(lines(mount));
				break;
			}
		}

		height = package_tooltip::height(contents.size(), desc_height, mount_height);

		int32_t sum_regular = 0, sum_paid = 0;

		for (size_t i = 0; i < contents.size(); i++)
		{
			const cash_catalog::Commodity &content = *contents[i];
			const ItemData &item = ItemData::get(content.item_id);
			int32_t regular = cash_catalog::regular_price(all, content);
			int16_t y = package_tooltip::row_y(i, desc_height);

			Row row;
			row.icon = item.get_icon(false);
			row.name = label(Color::Name::WHITE, item.get_name());
			row.price = label(Color::Name::WHITE, package_tooltip::price(content.count, regular));
			row.discount = content.price == regular
				? Text()
				: label(Color::Name::WHITE, package_tooltip::price(1, content.price));
			row.period = content.period == 0
				? Text()
				: label(Color::Name::WHITE, "AVAILABLE FOR " + std::to_string(content.period) + " DAYS.");
			row.y = y;
			row.name_y = package_tooltip::name_y(y, content.period);
			row.price_y = package_tooltip::price_y(y, content.period);
			row.percent = package_tooltip::percent(regular, content.price);
			rows.push_back(row);

			sum_regular += regular;
			sum_paid += content.price;
		}

		footer_y = package_tooltip::row_y(rows.size(), desc_height);
		total = label(Color::Name::DISCOUNTRED, package_tooltip::price(1, sum_regular));
		total_paid = label(Color::Name::WHITE, package_tooltip::price(1, sum_paid));
		total_percent = package_tooltip::percent(sum_regular, sum_paid);
	}

	void PackageTooltip::reset()
	{
		item_id = 0;
		rows.clear();
	}

	void PackageTooltip::draw(Point<int16_t> position) const
	{
		if (item_id == 0)
			return;

		// v83 keeps the box off the screen's last column and row, then off
		// its left and top.
		int16_t x = position.x(), y = position.y();
		int16_t screen_width = Constants::Constants::get().get_viewwidth();
		int16_t screen_height = Constants::Constants::get().get_viewheight();

		if (x + width > screen_width - 1)
			x = screen_width - 1 - width;

		if (y + height > screen_height - 1)
			y = screen_height - 1 - height;

		Point<int16_t> at(std::max<int16_t>(x, 0), std::max<int16_t>(y, 0));
		GraphicsGL &gl = GraphicsGL::get();
		auto box = [&](int16_t left, int16_t top, int16_t w, int16_t h)
		{
			gl.drawrectangle(at.x() + left, at.y() + top, w, h, 0.0f, 0.0f, FILL_BLUE, FILL_ALPHA);
		};
		// A white line a pixel high.
		auto rule = [&](int16_t left, int16_t top, int16_t w)
		{
			gl.drawrectangle(at.x() + left, at.y() + top, w, 1, 1.0f, 1.0f, 1.0f, 1.0f);
		};
		auto text = [&](const Text &t, int16_t left, int16_t top)
		{
			t.draw(at + Point<int16_t>(left, text_y(top)));
		};

		box(1, 0, width - 2, 1);
		box(0, 1, width, height - 2);
		box(1, height - 1, width - 2, 1);

		text(title, (width - title.width()) / 2, 8);
		text(desc, 11, package_tooltip::DESC_Y);

		using package_tooltip::ICON_X;
		using package_tooltip::NAME_X;

		for (const Row &row : rows)
		{
			Point<int16_t> icon_box = at + Point<int16_t>(ICON_X, row.y + 13);
			background.draw(icon_box);
			Point<int16_t> icon_size = row.icon.get_dimensions();
			row.icon.draw(icon_box + Point<int16_t>((ICON_BOX - icon_size.x()) / 2, (ICON_BOX - icon_size.y()) / 2)
				+ row.icon.get_origin());

			text(row.name, NAME_X, row.name_y);
			text(row.price, NAME_X, row.price_y);

			// A content dearer on its own is struck through, with what it
			// costs in the package and its discount.
			if (!row.discount.empty())
			{
				int16_t struck = row.price.width();
				rule(NAME_X, row.price_y + STRIKE_Y, struck + 2);
				int16_t arrow_x = NAME_X + struck + ARROW_GAP;
				arrow.draw(at + Point<int16_t>(arrow_x, row.price_y + ARROW_Y));
				text(row.discount, arrow_x + ARROW_ADVANCE, row.price_y);
				draw_badge(at + Point<int16_t>(width - BADGE_RIGHT, row.name_y - 1), row.percent);
			}

			text(row.period, NAME_X, package_tooltip::period_y(row.y));
			line.draw(at + Point<int16_t>(ICON_X, row.y + package_tooltip::ROW_HEIGHT));
		}

		// The footer: the contents' total on their own, struck by an
		// arrow to what the package costs, with the discount.
		rule(ICON_X, footer_y, width - 20);
		total_label.draw(at + Point<int16_t>(9, footer_y + 12));
		int16_t total_x = total_label.width() + ARROW_ADVANCE;
		text(total, total_x, footer_y + 11);
		int16_t arrow_x = total_x + total.width() + 5;
		arrow.draw(at + Point<int16_t>(arrow_x, footer_y + 12));
		text(total_paid, arrow_x + ARROW_ADVANCE, footer_y + 11);
		draw_badge(at + Point<int16_t>(width - BADGE_RIGHT, footer_y + 10), total_percent);
	}

	// FUN_008ef68f: "w", the percentage's digits, then "e" ("% OFF").
	void PackageTooltip::draw_badge(Point<int16_t> position, int32_t percent) const
	{
		badge_left.draw(position);
		position.shift_x(badge_left.width());

		for (int32_t digit : package_tooltip::digits(percent))
		{
			badge_digits[digit].draw(position);
			position.shift_x(DIGIT_PITCH);
		}

		badge_right.draw(position);
	}
}
