#include "UIItemSearchResult.h"

#include "UINotice.h"

#include "../UI.h"

#include "../Components/MapleButton.h"

#include "../../Configuration.h"
#include "../../Constants.h"
#include "../../Data/ItemData.h"
#include "../../Net/Packets/InventoryPackets.h"
#include "../../Util/Misc.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

#include <algorithm>

namespace ms
{
	namespace
	{
		// The window (FUN_008a26d6), against its top-left corner.
		const Point<int16_t> OK = { 190, 405 };
		const Point<int16_t> PREV = { 137, 368 };
		const Point<int16_t> NEXT = { 289, 368 };
		const Point<int16_t> PAGE = { 205, 369 };
		constexpr int16_t ROWS = 8;
		constexpr int16_t GO_X = 368, GO_TOP = 175;
		// The rows (FUN_008a2a14): names clipped to 60 pixels, numbers
		// right-aligned to their columns.
		constexpr int16_t ROW_TOP = 176, ROW_STEP = 22;
		constexpr int16_t OWNER_X = 21, TITLE_X = 87, NAME_WIDTH = 60;
		constexpr int16_t PER_BUNDLE_X = 180, PRICE_X = 255, BUNDLES_X = 287, INFO_X = 304, STATUS_X = 373;
		// The text box: what was searched, how many shops, and when GO goes.
		constexpr int16_t HEADER_X = 20, HEADER_Y[] = { 85, 99, 113 };

		// Where the tooltips are (FUN_008a45b9): rows count from 174 but
		// start at 176.
		constexpr int16_t COLUMN_TOP = 155, COLUMN_BOTTOM = 175, ROW_HIT = 174;

		// How far down to draw Arial 11 whose top v83 puts at y.
		int16_t text_y(int16_t y)
		{
			static const int16_t line = Text(Text::Font::A11M, Text::Alignment::LEFT, Color::Name::WHITE, "|").height();

			return y + 10 - line;
		}

		Text text(const std::string &str, Color::Name color = Color::Name::BLACK,
				  Text::Alignment alignment = Text::Alignment::LEFT)
		{
			return Text(Text::Font::A11M, alignment, color, str);
		}

		std::string with_commas(int32_t number)
		{
			std::string digits = std::to_string(number);
			string_format::split_number(digits);

			return digits;
		}
	}

	void UIItemSearchResult::open(owl::Results &&results)
	{
		UI::get().emplace<UIItemSearchResult>(std::move(results));
	}

	UIItemSearchResult::UIItemSearchResult(owl::Results &&r) : UIElement(Point<int16_t>(), Point<int16_t>()),
															   results(std::move(r))
	{
		nl::node src = NxFiles::ui_classic["UIWindow.img"]["itemSearch"];

		sprites.emplace_back(src["resultback"]);
		info = src["BtInfo"];
		dimension = Texture(src["resultback"]).get_dimensions();
		position = Point<int16_t>((Constants::Constants::get().get_viewwidth() - dimension.x()) / 2,
								  (Constants::Constants::get().get_viewheight() - dimension.y()) / 2);

		buttons[Buttons::BT_OK] = std::make_unique<MapleButton>(src["BtOK"], OK);
		buttons[Buttons::BT_PREV] = std::make_unique<MapleButton>(src["BtLeft"], PREV);
		buttons[Buttons::BT_NEXT] = std::make_unique<MapleButton>(src["BtRight"], NEXT);

		for (int16_t i = 0; i < ROWS; i++)
			buttons[Buttons::BT_GO + i] = std::make_unique<MapleButton>(
					src["BtLink"], Point<int16_t>(GO_X, GO_TOP + ROW_STEP * i));

		// SP 0xe30 to 0xe34, the name in red Arial 12.
		auto add = [&](size_t line, int16_t &x, Text piece)
		{
			int16_t width = piece.width();
			header.emplace_back(Point<int16_t>(x, text_y(HEADER_Y[line])), std::move(piece));
			x += width + 4;
		};

		int16_t x = HEADER_X;
		add(0, x, text("Search results for"));
		add(0, x, Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::RED,
					   ItemData::get(results.itemid).get_name()));
		add(0, x, text("that you entered."));

		x = HEADER_X;
		add(1, x, text("Total of"));
		add(1, x, text(std::to_string(results.rows.size()), Color::Name::RED));
		add(1, x, text("results available."));

		x = HEADER_X;
		add(2, x, text("You may not use \"GO\" if the store is either on a different channel or closed."));

		pages = static_cast<int16_t>(std::max<size_t>(1, (results.rows.size() + ROWS - 1) / ROWS));
		page_text = Text(Text::Font::A11M, Text::Alignment::LEFT, Color::Name::BLACK);
		set_page(0);
	}

	void UIItemSearchResult::draw(float alpha) const
	{
		UIElement::draw_sprites(alpha);

		for (const auto &[pos, piece] : header)
			piece.draw(position + pos);

		for (size_t i = 0; i < lines.size(); i++)
		{
			const Line &line = lines[i];
			int16_t y = text_y(ROW_TOP + ROW_STEP * static_cast<int16_t>(i) + 2);

			line.owner.draw_between(position + Point<int16_t>(OWNER_X, y),
									Range<int16_t>(position.x() + OWNER_X, position.x() + OWNER_X + NAME_WIDTH));
			line.title.draw_between(position + Point<int16_t>(TITLE_X, y),
									Range<int16_t>(position.x() + TITLE_X, position.x() + TITLE_X + NAME_WIDTH));
			line.per_bundle.draw(position + Point<int16_t>(PER_BUNDLE_X, y));
			line.price.draw(position + Point<int16_t>(PRICE_X, y));
			line.bundles.draw(position + Point<int16_t>(BUNDLES_X, y));
			line.status.draw(position + Point<int16_t>(STATUS_X, y));

			if (line.equip)
				info.draw(position + Point<int16_t>(INFO_X, ROW_TOP + ROW_STEP * static_cast<int16_t>(i) + 2));
		}

		page_text.draw(position + Point<int16_t>(PAGE.x(), text_y(PAGE.y())));

		UIElement::draw_buttons(alpha);
	}

	Cursor::State UIItemSearchResult::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		Cursor::State state = UIElement::send_cursor(clicked, cursorpos);

		show_tooltip(cursorpos);

		return state;
	}

	void UIItemSearchResult::remove_cursor()
	{
		UIElement::remove_cursor();

		UI::get().clear_tooltip(Tooltip::Parent::ITEMSEARCHRESULT);
	}

	void UIItemSearchResult::send_key(int32_t, bool pressed, bool escape)
	{
		if (pressed && escape)
			close();
	}

	UIElement::Type UIItemSearchResult::get_type() const
	{
		return TYPE;
	}

	void UIItemSearchResult::go_result(int8_t code)
	{
		if (code == owl::GO)
		{
			close();

			return;
		}

		// A closed shop greys out its row.
		if (code == owl::CLOSED && going >= 0 && going < static_cast<int32_t>(results.rows.size()))
			results.rows[going].channel = 0xff;

		going = -1;
		set_page(page);

		if (std::string notice = owl::go_text(code); !notice.empty())
			UI::get().emplace<UIOk>(notice, [](bool) {});
	}

	void UIItemSearchResult::close()
	{
		if (!is_active())
			return;

		UI::get().clear_tooltip(Tooltip::Parent::ITEMSEARCHRESULT);
		deactivate();
	}

	Button::State UIItemSearchResult::button_pressed(uint16_t buttonid)
	{
		switch (buttonid)
		{
			case Buttons::BT_OK:
				close();
				break;
			case Buttons::BT_PREV:
				set_page(page - 1);

				// set_page leaves the page buttons as they should be.
				return Button::State::IDENTITY;
			case Buttons::BT_NEXT:
				set_page(page + 1);

				return Button::State::IDENTITY;
			default:
				if (buttonid >= Buttons::BT_GO && buttonid < Buttons::BT_GO + ROWS && going < 0)
				{
					int32_t index = page * ROWS + (buttonid - Buttons::BT_GO);

					if (index < static_cast<int32_t>(results.rows.size()) && can_go(results.rows[index]))
					{
						going = index;
						OwlGoPacket(results.rows[index].room, results.rows[index].mapid).dispatch();
					}
				}

				break;
		}

		return Button::State::NORMAL;
	}

	void UIItemSearchResult::set_page(int16_t p)
	{
		page = std::clamp<int16_t>(p, 0, pages - 1);
		lines.clear();

		buttons[Buttons::BT_PREV]->set_state(page > 0 ? Button::State::NORMAL : Button::State::DISABLED);
		buttons[Buttons::BT_NEXT]->set_state(page < pages - 1 ? Button::State::NORMAL : Button::State::DISABLED);
		page_text.change_text("[" + std::to_string(page + 1) + "/" + std::to_string(pages) + "]");

		for (int16_t i = 0; i < ROWS; i++)
		{
			size_t index = static_cast<size_t>(page * ROWS + i);
			bool shown = index < results.rows.size();
			bool go = shown && can_go(results.rows[index]);

			// FUN_008a4c6c: every GO is there, greyed out where it can't go.
			buttons[Buttons::BT_GO + i]->set_state(go ? Button::State::NORMAL : Button::State::DISABLED);

			if (!shown)
				continue;

			const owl::Row &row = results.rows[index];
			std::string status;

			// SP 0xe36 and 0xe37 where GO isn't offered.
			if (!go)
				status = row.channel == 0xff ? "Store Closed" : "Can't Go";

			lines.push_back({ text(row.owner), text(row.title),
							  text(std::to_string(row.per_bundle), Color::Name::BLACK, Text::Alignment::RIGHT),
							  text(with_commas(row.price), Color::Name::BLACK, Text::Alignment::RIGHT),
							  text(std::to_string(row.bundles), Color::Name::BLACK, Text::Alignment::RIGHT),
							  text(status, Color::Name::RED), row.equip.has_value() });
		}
	}

	bool UIItemSearchResult::can_go(const owl::Row &row) const
	{
		return row.channel == Configuration::get().get_channelid();
	}

	void UIItemSearchResult::show_tooltip(Point<int16_t> cursorpos)
	{
		Point<int16_t> pos = cursorpos - position;
		int16_t x = pos.x(), y = pos.y();

		if (y >= COLUMN_TOP && y < COLUMN_BOTTOM)
		{
			// SP 0xe42 to 0xe44, over the numbers' columns.
			if (x >= 151 && x < 181)
				return UI::get().show_text(Tooltip::Parent::ITEMSEARCHRESULT, "# Item Per Bundle");

			if (x >= 206 && x < 236)
				return UI::get().show_text(Tooltip::Parent::ITEMSEARCHRESULT, "Price Per Bundle");

			if (x >= 258 && x < 288)
				return UI::get().show_text(Tooltip::Parent::ITEMSEARCHRESULT, "Total # Sold");
		}
		else if (y >= ROW_TOP)
		{
			int32_t row = (y - ROW_HIT) / ROW_STEP;
			int32_t index = page * ROWS + row;

			if (row < static_cast<int32_t>(lines.size()) && index < static_cast<int32_t>(results.rows.size()))
			{
				const owl::Row &entry = results.rows[index];

				if (x >= OWNER_X && x < TITLE_X)
					return UI::get().show_text(Tooltip::Parent::ITEMSEARCHRESULT, entry.owner);

				if (x >= TITLE_X && x < TITLE_X + 50)
					return UI::get().show_text(Tooltip::Parent::ITEMSEARCHRESULT, entry.title);

				if (x >= INFO_X && x < INFO_X + 57)
				{
					if (entry.equip)
						return UI::get().show_item(Tooltip::Parent::ITEMSEARCHRESULT, entry.equip->itemid,
												   entry.equip->expiration, entry.equip->flags);

					return UI::get().show_item(Tooltip::Parent::ITEMSEARCHRESULT, results.itemid);
				}

				// SP 0xe38: where a shop on another channel is. The Free
				// Market's rooms are numbered by their maps.
				if (x >= 370 && x < 424 && entry.channel != 0xff && !can_go(entry))
					return UI::get().show_text(Tooltip::Parent::ITEMSEARCHRESULT,
											   "This store is open at Room " + std::to_string(entry.mapid % 100)
											   + " on Channel " + std::to_string(entry.channel + 1) + ".");
			}
		}

		UI::get().clear_tooltip(Tooltip::Parent::ITEMSEARCHRESULT);
	}
}
