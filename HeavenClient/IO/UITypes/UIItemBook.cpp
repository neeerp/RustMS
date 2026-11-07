#include "UIItemBook.h"

#include "../KeyAction.h"
#include "../UI.h"

#include "../Components/MapleButton.h"

#include "../../Audio/Audio.h"
#include "../../Graphics/Geometry.h"
#include "../../Util/NpcTextMarkup.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// Where the window always opens (0x9de4d2 at 0x461b8c), not centred.
		constexpr Point<int16_t> POSITION = Point<int16_t>(190, 148);
		constexpr Point<int16_t> DIMENSION = Point<int16_t>(478, 353);
		// The left page's text from the window's top left, the right page's 220 further (0x4628dd), each 190 wide
		// (0x462e0c).
		constexpr Point<int16_t> TEXT = Point<int16_t>(35, 30);
		constexpr int16_t PAGE_STEP = 220;
		constexpr int16_t PAGE_WIDTH = 190;
		// Between one entry of a page and the next (0x463100).
		constexpr int16_t ENTRY_GAP = 10;
		// The page numbers' line (0x462c1b).
		constexpr int16_t NUMBER_Y = 293;
		// The buttons (0x4624f4, 0x4625a5, 0x462622).
		constexpr Point<int16_t> PREV_BUTTON = Point<int16_t>(34, 287);
		constexpr Point<int16_t> NEXT_BUTTON = Point<int16_t>(403, 287);
		constexpr Point<int16_t> CLOSE_BUTTON = Point<int16_t>(430, 10);
		// A link's dot and the room it takes, as in NPC text: the same layout (0x99dd54).
		constexpr int16_t DOT_X = 10;
		constexpr int16_t LABEL_X = 18;

		constexpr Text::Font FONT = Text::Font::A12M;

		nl::node book_of(int32_t item_id)
		{
			return nl::nx::item["Etc"]["0" + std::to_string(item_id / 10000) + ".img"]["0" + std::to_string(item_id)]["book"];
		}

		// An entry's `align`, an int or a string (0x414d40).
		item_book::Align align_of(nl::node align)
		{
			if (align.data_type() == nl::node::type::string)
				return item_book::align(align.get_string());

			return item_book::align(align.get_integer());
		}

		Point<int16_t> page_origin(int16_t side)
		{
			return TEXT + Point<int16_t>(side * PAGE_STEP, 0);
		}
	}

	void UIItemBook::open(int32_t item_id)
	{
		if (auto window = UI::get().get_element<UIItemBook>(); window && window->is_active())
		{
			// The same book does nothing (0x462cd8); another shows in this window, without a sound.
			if (window->item_id != item_id)
				window->set_item(item_id);

			return;
		}

		// The state's emplace plays MenuUp.
		UI::get().emplace<UIItemBook>(item_id);
	}

	UIItemBook::UIItemBook(int32_t id)
			: UIElement(POSITION, DIMENSION), item_id(0), last(-1), page(-1), dragged(false)
	{
		// Its art is v83's: UI.nx has 1 by 1 buttons here.
		nl::node src = NxFiles::ui_classic["UIWindow.img"]["Book"];

		background = src["backgrnd"];
		buttons[Buttons::PREV] = std::make_unique<MapleButton>(src["BtPrev"], PREV_BUTTON);
		buttons[Buttons::NEXT] = std::make_unique<MapleButton>(src["BtNext"], NEXT_BUTTON);
		buttons[Buttons::CLOSE] = std::make_unique<MapleButton>(src["BtClose"], CLOSE_BUTTON);

		nl::node UtilDlgEx = NxFiles::ui_classic["UIWindow.img"]["UtilDlgEx"];
		dot = UtilDlgEx["dot0"];
		dot_hovered = UtilDlgEx["dot1"];

		line = Text(FONT, Text::Alignment::LEFT, Color::Name::BLACK, "|").height();

		set_item(id);
	}

	void UIItemBook::set_item(int32_t id)
	{
		// An item without a book shows an empty window (0x462d72).
		item_id = id;
		book = book_of(id);
		last = static_cast<int32_t>(book.size()) - 1;
		page = -1;
		set_page(0);
	}

	void UIItemBook::set_page(int32_t first)
	{
		page = first;
		hovered = LinkAt();

		for (int16_t side = 0; side < 2; side++)
		{
			Page &shown = pages[side];
			int32_t index = first + side;
			nl::node node = book[std::to_string(index)];

			shown = Page();

			// A missing right page is left blank, without a number (0x462ed0).
			if (index > last || !node)
				continue;

			shown.shown = true;
			shown.number = Text(FONT, Text::Alignment::CENTER, Color::Name::BLACK, item_book::page_label(index, last));

			int16_t top = 0;

			for (int32_t k = 0; nl::node part = node[std::to_string(k)]; k++)
			{
				std::string text = part["text"];

				// Empty text lays out nothing and adds no gap.
				if (text.empty())
					continue;

				Entry laid = entry(text, align_of(part["align"]), top);
				top = laid.at.y() + laid.text.height() + ENTRY_GAP;
				shown.entries.push_back(std::move(laid));
			}
		}

		// 0x46320d.
		buttons[Buttons::PREV]->set_state(item_book::has_previous(page) ? Button::State::NORMAL : Button::State::DISABLED);
		buttons[Buttons::NEXT]->set_state(item_book::has_next(page, last) ? Button::State::NORMAL : Button::State::DISABLED);
	}

	UIItemBook::Entry UIItemBook::entry(const std::string &source, item_book::Align align, int16_t top) const
	{
		Entry laid;

		// The shared text layout (0x99dd54): names, icons and links as in NPC text; the data's "\n" is two
		// characters, which the layout takes as a line break.
		std::vector<NpcTextMarkup::InlineImage> tags;
		std::string text = NpcTextMarkup::expand(source, UINpcTalk::markup_lookups(), &tags);
		std::vector<int32_t> ids;
		text = NpcTextMarkup::mark_choices(text, ids);

		for (const auto &tag : tags)
			laid.images.push_back(UINpcTalk::inline_image(tag));

		// Images and a link's ends are boxes: its start leaves room for its dot, its end takes none.
		std::vector<Point<int16_t>> boxes;
		std::vector<std::pair<size_t, size_t>> marks;
		size_t next_image = 0;

		for (char &c : text)
		{
			if (c == Text::BOX)
			{
				if (next_image < laid.images.size())
				{
					laid.images[next_image].box = boxes.size();
					boxes.push_back(laid.images[next_image++].size);
				} else
				{
					boxes.emplace_back(0, 0);
				}
			} else if (c == NpcTextMarkup::CHOICE)
			{
				marks.emplace_back(boxes.size(), 0);
				boxes.emplace_back(LABEL_X, 0);
				c = Text::BOX;
			} else if (c == NpcTextMarkup::CHOICE_END)
			{
				if (!marks.empty())
					marks.back().second = boxes.size();

				boxes.emplace_back(0, 0);
				c = Text::BOX;
			}
		}

		// Each line is centred or put right in the page's width (0x463110).
		Text::Alignment alignment = Text::Alignment::LEFT;
		int16_t x = 0;

		switch (align)
		{
			case item_book::Align::CENTER:
				alignment = Text::Alignment::CENTER;
				x = PAGE_WIDTH / 2;
				break;
			case item_book::Align::RIGHT:
				alignment = Text::Alignment::RIGHT;
				x = PAGE_WIDTH;
				break;
			default:
				break;
		}

		laid.text = Text(FONT, alignment, Color::Name::BLACK, text, PAGE_WIDTH);
		laid.text.set_boxes(boxes);
		laid.at = Point<int16_t>(x, top);

		for (size_t i = 0; i < marks.size() && i < ids.size(); i++)
			laid.links.push_back({ ids[i], laid.text.box_position(marks[i].first),
								   laid.text.box_position(marks[i].second) });

		return laid;
	}

	void UIItemBook::draw(float alpha) const
	{
		background.draw(position);

		for (int16_t side = 0; side < 2; side++)
		{
			const Page &shown = pages[side];

			if (!shown.shown)
				continue;

			Point<int16_t> origin = position + page_origin(side);

			for (size_t e = 0; e < shown.entries.size(); e++)
			{
				const Entry &laid = shown.entries[e];
				Point<int16_t> at = origin + laid.at;

				for (const auto &image : laid.images)
					image.texture.draw(at + laid.text.box_position(image.box) + image.anchor);

				laid.text.draw(at);

				// Each link has its dot, the hovered one's lit and underlined (0x46292a).
				for (size_t l = 0; l < laid.links.size(); l++)
				{
					const Link &link = laid.links[l];
					bool lit = hovered.side == side && hovered.entry == e && hovered.link == l;
					Point<int16_t> start = at + link.start;

					(lit ? dot_hovered : dot).draw(start + Point<int16_t>(DOT_X, -8));

					if (lit)
					{
						int16_t to = link.end.y() == link.start.y() ? at.x() + link.end.x() : origin.x() + PAGE_WIDTH;

						ColorBox(to - start.x() - DOT_X, 1, Color::Name::BLACK, 1.0f)
							.draw(start + Point<int16_t>(DOT_X, 2));
					}
				}
			}

			shown.number.draw(origin + Point<int16_t>(PAGE_WIDTH / 2, NUMBER_Y - TEXT.y()));
		}

		UIElement::draw(alpha);
	}

	bool UIItemBook::link_contains(const Link &link, int16_t at, Point<int16_t> cursor) const
	{
		// A link's points are from its entry's `at`, which is the page's centre or right edge when it is aligned so.
		int16_t left = -at;
		int16_t right = PAGE_WIDTH - at;

		Point<int16_t> start = link.start;
		Point<int16_t> end = link.end;

		if (start.y() == end.y())
			return Rectangle<int16_t>(start.x(), end.x(), start.y() - line + 4, start.y() + 4).contains(cursor);

		// A link that wraps answers on all its lines.
		int16_t last_top = end.y() - line + 4;

		return Rectangle<int16_t>(start.x(), right, start.y() - line + 4, start.y() + 4).contains(cursor)
			|| Rectangle<int16_t>(left, right, start.y() + 4, last_top).contains(cursor)
			|| Rectangle<int16_t>(left, end.x(), last_top, end.y() + 4).contains(cursor);
	}

	UIItemBook::LinkAt UIItemBook::link_at(Point<int16_t> cursor) const
	{
		for (int16_t side = 0; side < 2; side++)
		{
			const Page &shown = pages[side];

			for (size_t e = 0; e < shown.entries.size(); e++)
			{
				const Entry &laid = shown.entries[e];
				Point<int16_t> relative = cursor - page_origin(side) - laid.at;

				for (size_t l = 0; l < laid.links.size(); l++)
					if (link_contains(laid.links[l], laid.at.x(), relative))
						return { side, e, l };
			}
		}

		return LinkAt();
	}

	void UIItemBook::remove_cursor()
	{
		UIElement::remove_cursor();

		hovered = LinkAt();
		dragged = false;
	}

	Cursor::State UIItemBook::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		if (dragged)
		{
			if (clicked)
			{
				position = cursorpos - drag_from;

				return Cursor::State::CLICKING;
			}

			dragged = false;
		}

		Point<int16_t> cursor = cursorpos - position;
		bool press = clicked && UI::get().is_cursor_press();

		hovered = link_at(cursor);

		// A press on a link turns to its page's spread (0x462770).
		if (hovered.side >= 0)
		{
			if (press)
			{
				const Link &link = pages[hovered.side].entries[hovered.entry].links[hovered.link];
				int32_t to = item_book::link_target(link.page, last);

				if (to >= 0)
					set_page(to);
			}

			return Cursor::State::CANCLICK;
		}

		Cursor::State ret = UIElement::send_cursor(clicked, cursorpos);

		// Anywhere else that isn't a button, even a disabled one, moves it (0x4627b1).
		bool on_button = is_holding_button();

		for (auto &button : buttons)
			on_button = on_button || button.second->bounds(position).contains(cursorpos);

		if (press && !on_button && Rectangle<int16_t>(Point<int16_t>(), dimension).contains(cursor))
		{
			drag_from = cursor;
			dragged = true;
		}

		return ret;
	}

	void UIItemBook::send_key(int32_t keycode, bool pressed, bool escape)
	{
		// Esc and Enter close it, on the key going down (0x462738).
		if (pressed && (escape || keycode == KeyAction::Id::RETURN))
			close();
	}

	UIElement::Type UIItemBook::get_type() const
	{
		return TYPE;
	}

	Button::State UIItemBook::button_pressed(uint16_t buttonid)
	{
		switch (buttonid)
		{
			case Buttons::PREV:
				if (int32_t to = item_book::previous(page); to >= 0)
					set_page(to);

				break;
			case Buttons::NEXT:
				if (int32_t to = item_book::next(page, last); to >= 0)
					set_page(to);

				break;
			case Buttons::CLOSE:
				close();

				return Button::State::NORMAL;
		}

		// The turn decided whether the button is still enabled.
		return buttons[buttonid]->get_state() == Button::State::DISABLED ? Button::State::DISABLED
																		 : Button::State::MOUSEOVER;
	}

	void UIItemBook::close()
	{
		// 0xa10679: closing always ends the window; it opens again at its first spread.
		Sound(Sound::Name::MENUUP).play();
		deactivate();
	}
}
