#include "UIItemSearch.h"

#include "UINotice.h"

#include "../KeyAction.h"
#include "../UI.h"

#include "../Components/MapleButton.h"

#include "../../Constants.h"
#include "../../Audio/Audio.h"
#include "../../Data/ItemData.h"
#include "../../Data/ItemNames.h"
#include "../../Gameplay/Stage.h"
#include "../../Graphics/GraphicsGL.h"
#include "../../Net/Packets/InventoryPackets.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

#include <algorithm>

namespace ms
{
	namespace
	{
		// The window (FUN_008a1364), against its top-left corner: dragged by
		// the strip with its title.
		const Point<int16_t> POPULAR = { 185, 220 };
		const Point<int16_t> CATEGORIES = { 185, 243 };
		const Point<int16_t> SEARCH = { 175, 76 };
		const Point<int16_t> CLOSE = { 229, 8 };
		const Rectangle<int16_t> EDIT = Rectangle<int16_t>(18, 18 + 150, 78, 78 + 13);
		constexpr size_t EDIT_LIMIT = 256;
		constexpr int16_t DRAG_HEIGHT = 24;

		// A panel slides out from under the window's right edge, a pixel a
		// millisecond (FUN_008a1e4d), with its own buttons (FUN_0089cf9b).
		const Point<int16_t> PANEL = { 243, 0 };
		constexpr int16_t SLIDE_STEP = Constants::TIMESTEP;
		const Point<int16_t> HIDE = { 245, 7 };
		const Point<int16_t> PREV = { 230, 252 };
		const Point<int16_t> NEXT = { 245, 252 };
		const Point<int16_t> CHOOSE = { 180, 249 };
		// The order switch, ticked for the dearest first, and its caption
		// (FUN_0089d4a4, FUN_0089e4f2).
		const Rectangle<int16_t> ORDER = Rectangle<int16_t>(16, 16 + 110, 249, 249 + 14);
		constexpr int16_t ORDER_CAPTION_X = 35;

		// Rows light up behind the one under the cursor and the one chosen.
		constexpr int16_t ROW_LEFT = 15, ROW_WIDTH = 250;
		constexpr Color SELECTED = Color(0x5B7B9EFF);
		constexpr Color HOVERED = Color(0xBBC9D9FF);

		// The items searched for most, five a page with their icons
		// (FUN_0089e4f2).
		constexpr int16_t POPULAR_ROWS = 5, POPULAR_TOP = 40, POPULAR_STEP = 40;
		// The items found, ten a page (FUN_0089d4a4).
		constexpr int16_t FOUND_ROWS = 10, FOUND_TOP = 43, FOUND_STEP = 20;
		// The category tree (FUN_0089f618).
		constexpr int16_t TREE_TOP = 40, TREE_STEP = 13, HEADING_X = 30, CATEGORY_X = 50;

		// SP 0xe2d, 0xe2e and 0xe2f.
		std::string question(const std::string &name, bool descending)
		{
			return "Would you like to search for\\n[" + name + "]?\\nThe Owl of Minerva will disappear\\n"
				   + (descending ? "Show 200 results in descending order" : "Show 200 results in ascending order")
				   + "\\nif there's a search result after pressing OK.";
		}

		// How far down to draw Arial 12 whose top v83 puts at y: HC puts a
		// line's baseline a full line below where it's drawn.
		int16_t text_y(int16_t y)
		{
			static const int16_t line = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE, "|").height();

			return y + 11 - line;
		}

		// A category takes the items of one or two families of ids
		// (FUN_008a0c0e, FUN_008a0c50).
		constexpr int32_t family(int32_t n)
		{
			return n * 10000;
		}
	}

	void UIItemSearch::open(int16_t slot, int32_t owl_id)
	{
		if (auto window = UI::get().get_element<UIItemSearch>(); window && window->is_active())
			return;

		UI::get().emplace<UIItemSearch>(slot, owl_id);
	}

	UIItemSearch::UIItemSearch(int16_t s, int32_t o) : UIElement(Point<int16_t>(), Point<int16_t>()), slot(s), owl_id(o)
	{
		nl::node src = NxFiles::ui_classic["UIWindow.img"]["itemSearch"];

		background = src["backgrnd"];
		panel_background = src["categoryback"];
		order_icon = src["icon0"];
		dimension = background.get_dimensions();
		position = Point<int16_t>((Constants::Constants::get().get_viewwidth() - dimension.x()) / 2,
								  (Constants::Constants::get().get_viewheight() - dimension.y()) / 2);

		buttons[Buttons::BT_POPULAR] = std::make_unique<MapleButton>(src["BtGo"], POPULAR);
		buttons[Buttons::BT_CATEGORIES] = std::make_unique<MapleButton>(src["BtGo"], CATEGORIES);
		buttons[Buttons::BT_SEARCH] = std::make_unique<MapleButton>(src["BtSearch"], SEARCH);
		buttons[Buttons::BT_CLOSE] = std::make_unique<MapleButton>(NxFiles::ui_classic["Basic.img"]["BtClose"], CLOSE);
		buttons[Buttons::BT_HIDE] = std::make_unique<MapleButton>(NxFiles::ui_classic["Basic.img"]["BtHide"],
																  PANEL + HIDE);
		buttons[Buttons::BT_PREV] = std::make_unique<MapleButton>(src["BtLeft"], PANEL + PREV);
		buttons[Buttons::BT_NEXT] = std::make_unique<MapleButton>(src["BtRight"], PANEL + NEXT);
		buttons[Buttons::BT_CHOOSE] = std::make_unique<MapleButton>(src["BtSearch"], PANEL + CHOOSE);

		edit = Textfield(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK,
						 Rectangle<int16_t>(EDIT.left(), EDIT.right(), text_y(EDIT.top()), EDIT.bottom()), EDIT_LIMIT);

		edit.set_enter_callback(
				[&](std::string text)
				{
					search_name(text);
				}
		);

		edit.set_key_callback(
				KeyAction::Id::ESCAPE,
				[&]()
				{
					close();
				}
		);

		edit.set_state(Textfield::State::FOCUSED);

		// SP 0x131d.
		order = Text(Text::Font::A11M, Text::Alignment::LEFT, Color::Name::BLACK, "SORT BY PRICE");
		open_panel(Panel::NONE);

		// FUN_008a0e9a: the window asks for the items searched for most.
		OwlOpenPacket().dispatch();
	}

	void UIItemSearch::draw(float alpha) const
	{
		GraphicsGL &graphics = GraphicsGL::get();

		if (panel != Panel::NONE)
		{
			Point<int16_t> origin = position + panel_offset();
			panel_background.draw(origin);

			int16_t top = panel == Panel::POPULAR ? POPULAR_TOP : panel == Panel::RESULTS ? FOUND_TOP : TREE_TOP;
			int16_t step = panel == Panel::POPULAR ? POPULAR_STEP : panel == Panel::RESULTS ? FOUND_STEP : TREE_STEP;
			for (int16_t row = 0; row < rows_shown(); row++)
			{
				int16_t y = top + step * row;

				if (row == hovered || (row == selected && panel == Panel::RESULTS))
				{
					Color fill = row == selected && panel == Panel::RESULTS ? SELECTED : HOVERED;
					graphics.drawrectangle(origin.x() + ROW_LEFT, origin.y() + y, ROW_WIDTH, step, fill.r(), fill.g(),
										   fill.b(), fill.a());
				}

				switch (panel)
				{
					case Panel::POPULAR:
						ItemData::get(item_at(row)).get_icon(false).draw(origin + Point<int16_t>(20, y + 40));
						names[row].draw(origin + Point<int16_t>(70, text_y(y + 18)));
						break;
					case Panel::RESULTS:
						names[row].draw(origin + Point<int16_t>(20, text_y(y + 3)));
						break;
					default:
						names[row].draw(origin + Point<int16_t>(tree[row].heading ? HEADING_X : CATEGORY_X, text_y(y)));
						break;
				}
			}

			if (panel != Panel::CATEGORIES)
			{
				if (descending)
					order_icon.draw(origin + Point<int16_t>(ORDER.left(), ORDER.top() + 3));

				order.draw(origin + Point<int16_t>(ORDER_CAPTION_X, ORDER.top() - 1));
			}
		}

		background.draw(position);
		edit.draw(position);

		UIElement::draw_buttons(alpha);
	}

	void UIItemSearch::update()
	{
		UIElement::update();

		edit.update(position);

		if (hiding)
		{
			slide = std::max<int16_t>(0, slide - SLIDE_STEP);

			if (slide == 0)
				open_panel(Panel::NONE);
		}
		else if (panel != Panel::NONE)
		{
			slide = std::min<int16_t>(PANEL.x(), slide + SLIDE_STEP);
		}

		// The panel's buttons come with it once it is out.
		bool out = panel != Panel::NONE && !hiding && slide == PANEL.x();
		bool paged = panel == Panel::POPULAR || panel == Panel::RESULTS;
		buttons[Buttons::BT_HIDE]->set_active(out);
		buttons[Buttons::BT_PREV]->set_active(out && paged);
		buttons[Buttons::BT_NEXT]->set_active(out && paged);
		buttons[Buttons::BT_CHOOSE]->set_active(out && panel == Panel::RESULTS);
	}

	bool UIItemSearch::is_in_range(Point<int16_t> cursorpos) const
	{
		return Rectangle<int16_t>(position, position + dimension).contains(cursorpos)
			   || (panel != Panel::NONE && panel_bounds().contains(cursorpos));
	}

	Cursor::State UIItemSearch::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		// Rows and the order act as the button goes down.
		bool pressed = clicked && !mouse_down;
		mouse_down = clicked;

		if (dragged)
		{
			if (clicked)
			{
				position = cursorpos - cursoroffset;

				return Cursor::State::CLICKING;
			}

			dragged = false;
		}

		if (Cursor::State state = edit.send_cursor(cursorpos, clicked); state != Cursor::State::IDLE)
			return state;

		Cursor::State state = UIElement::send_cursor(clicked, cursorpos);

		if (is_holding_button())
		{
			hovered = -1;

			return state;
		}

		hovered = row_at(cursorpos);

		if (pressed && hovered >= 0)
		{
			int16_t row = hovered;

			switch (panel)
			{
				case Panel::POPULAR:
					// A row of the items searched for most asks at once.
					ask(item_at(row));
					break;
				case Panel::RESULTS:
					selected = row;
					buttons[Buttons::BT_CHOOSE]->set_state(Button::State::NORMAL);
					break;
				case Panel::CATEGORIES:
				{
					Category category = tree[row];

					if (category.opens >= 0)
					{
						part = category.opens;
						show(Panel::CATEGORIES);
					}
					else
					{
						search_category(category);
					}

					break;
				}
				default:
					break;
			}

			return Cursor::State::CLICKING;
		}

		if (pressed && panel != Panel::NONE && panel != Panel::CATEGORIES && !hiding
			&& order_bounds().contains(cursorpos))
		{
			descending = !descending;

			return Cursor::State::CLICKING;
		}

		Point<int16_t> pos = cursorpos - position;

		if (pressed && pos.x() >= 0 && pos.x() < dimension.x() && pos.y() >= 0 && pos.y() < DRAG_HEIGHT)
		{
			dragged = true;
			cursoroffset = pos;
		}

		return hovered >= 0 ? Cursor::State::CANCLICK : state;
	}

	void UIItemSearch::remove_cursor()
	{
		UIElement::remove_cursor();

		hovered = -1;
		mouse_down = false;
	}

	void UIItemSearch::send_key(int32_t, bool pressed, bool escape)
	{
		if (pressed && escape)
			close();
	}

	UIElement::Type UIItemSearch::get_type() const
	{
		return TYPE;
	}

	void UIItemSearch::set_popular(std::vector<int32_t> items)
	{
		popular = std::move(items);

		if (panel == Panel::POPULAR)
			set_page(0);
	}

	void UIItemSearch::close()
	{
		if (!is_active())
			return;

		edit.set_state(Textfield::State::DISABLED);
		deactivate();
	}

	Button::State UIItemSearch::button_pressed(uint16_t buttonid)
	{
		switch (buttonid)
		{
			case Buttons::BT_POPULAR:
				show(panel == Panel::POPULAR && !hiding ? Panel::NONE : Panel::POPULAR);
				break;
			case Buttons::BT_CATEGORIES:
				show(panel == Panel::CATEGORIES && !hiding ? Panel::NONE : Panel::CATEGORIES);
				break;
			case Buttons::BT_SEARCH:
				search_name(edit.get_text());
				break;
			case Buttons::BT_CLOSE:
				close();
				break;
			case Buttons::BT_HIDE:
				show(Panel::NONE);
				break;
			case Buttons::BT_PREV:
				set_page(page - 1);

				// set_page leaves the page buttons as they should be.
				return Button::State::IDENTITY;
			case Buttons::BT_NEXT:
				set_page(page + 1);

				return Button::State::IDENTITY;
			case Buttons::BT_CHOOSE:
				if (selected >= 0)
					ask(item_at(selected));

				return Button::State::IDENTITY;
			default:
				break;
		}

		return Button::State::NORMAL;
	}

	void UIItemSearch::show(Panel shown)
	{
		// FUN_008a166d: a panel going away slides back under the window
		// before it is gone; another one starts from under it.
		if (shown == Panel::NONE)
		{
			hiding = panel != Panel::NONE;
			hovered = -1;

			return;
		}

		if (shown != panel)
			slide = 0;

		hiding = false;
		open_panel(shown);
	}

	void UIItemSearch::open_panel(Panel shown)
	{
		panel = shown;
		part = shown == Panel::CATEGORIES ? part : 0;

		set_page(0);
	}

	void UIItemSearch::search_name(const std::string &text)
	{
		// FUN_008a229a: the text without its spaces, in lower case, in the
		// name of an item the client knows.
		std::string key = pets::search_key(text);

		if (key.empty())
			return;

		found.clear();

		for (const auto &[name, id] : item_search_names())
			if (name.find(key) != std::string::npos && ItemData::get(id).is_valid())
				found.push_back(id);

		show(Panel::RESULTS);
	}

	void UIItemSearch::search_category(const Category &category)
	{
		found.clear();

		for (const auto &[name, id] : item_search_names())
			if (((id >= category.low && id <= category.high) || (id >= category.low2 && id <= category.high2))
				&& ItemData::get(id).is_valid())
				found.push_back(id);

		std::sort(found.begin(), found.end());
		show(Panel::RESULTS);
	}

	void UIItemSearch::set_page(int16_t p)
	{
		int16_t per_page = panel == Panel::POPULAR ? POPULAR_ROWS : FOUND_ROWS;
		size_t count = panel == Panel::POPULAR ? popular.size() : found.size();
		auto pages = static_cast<int16_t>(std::max<size_t>(1, (count + per_page - 1) / per_page));

		page = std::clamp<int16_t>(p, 0, pages - 1);
		hovered = -1;
		selected = -1;

		buttons[Buttons::BT_PREV]->set_state(page > 0 ? Button::State::NORMAL : Button::State::DISABLED);
		buttons[Buttons::BT_NEXT]->set_state(page < pages - 1 ? Button::State::NORMAL : Button::State::DISABLED);
		buttons[Buttons::BT_CHOOSE]->set_state(Button::State::DISABLED);

		names.clear();

		if (panel == Panel::CATEGORIES)
		{
			tree = categories();

			for (const Category &category : tree)
				names.emplace_back(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, category.label);

			return;
		}

		for (int16_t row = 0; row < rows_shown(); row++)
			names.emplace_back(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK,
							   ItemData::get(item_at(row)).get_name());
	}

	void UIItemSearch::ask(int32_t item_id)
	{
		bool refocus = edit.get_state() == Textfield::State::FOCUSED;

		if (refocus)
			UI::get().remove_textfield();

		UI::get().emplace<UIYesNo>(
				question(ItemData::get(item_id).get_name(), descending),
				[refocus, item_id](bool yes)
				{
					auto window = UI::get().get_element<UIItemSearch>();

					if (!window || !window->is_active())
						return;

					// FUN_008a2407: one search each half second, the Owl used
					// up only if something turns up.
					if (yes
						&& Stage::get().get_player().request(
							UseOwlPacket(window->slot, window->owl_id, item_id, window->descending),
							ExclRequest::WAIT))
					{
						Sound(Sound::Name::MENUUP).play();
						window->close();

						return;
					}

					if (refocus)
						window->edit.set_state(Textfield::State::FOCUSED);
				}
		);
	}

	std::vector<UIItemSearch::Category> UIItemSearch::categories() const
	{
		// FUN_0089ec7c and FUN_008a0bd7: Eqp opens the armor and the two
		// kinds of weapons, each of which takes the armor's place.
		auto heading = [](const std::string &label, int8_t opens)
		{
			return Category { label, true, opens, 0, -1, 0, -1 };
		};

		auto one = [](const std::string &label, int32_t n)
		{
			return Category { label, false, -1, family(n), family(n) + 9999, 0, -1 };
		};

		auto two = [](const std::string &label, int32_t low, int32_t high, int32_t low2, int32_t high2)
		{
			return Category { label, false, -1, low, high, low2, high2 };
		};

		std::vector<Category> tree;
		tree.push_back(heading("Eqp", part == 0 || part == 2 || part == 3 ? 1 : 0));

		if (part == 1)
		{
			tree.push_back(one("HAT", 100));
			tree.push_back(one("EARRING", 103));
			tree.push_back(one("TOP", 104));
			tree.push_back(one("BOTTOM", 106));
			tree.push_back(one("Overall Armor", 105));
			tree.push_back(one("GLOVE", 108));
			tree.push_back(one("SHOES", 107));
			tree.push_back(one("SHIELD", 109));
			tree.push_back(one("CAPE", 110));
			tree.push_back(heading("One-handed Weapon", 4));
			tree.push_back(heading("Two-handed Weapon", 5));
		}
		else if (part == 4)
		{
			tree.push_back(one("ONE-HANDED SWORD", 130));
			tree.push_back(one("ONE-HANDED AXE", 131));
			tree.push_back(one("ONE_HANDED MACE", 132));
			tree.push_back(one("DAGGER", 133));
			tree.push_back(one("WAND", 137));
			tree.push_back(one("STAFF", 138));
		}
		else if (part == 5)
		{
			tree.push_back(one("TWO-HANDED SWORD", 140));
			tree.push_back(one("TWO-HANDED AXE", 141));
			tree.push_back(one("TWO-HANDED MACE", 142));
			tree.push_back(one("SPEAR", 143));
			tree.push_back(one("POLE ARM", 144));
			tree.push_back(one("BOW", 145));
			tree.push_back(one("CROSSBOW", 146));
			tree.push_back(one("CLAW", 147));
			tree.push_back(one("KNUCKLE", 148));
			tree.push_back(one("GUN", 149));
		}

		tree.push_back(heading("Use", part == 2 ? 0 : 2));

		if (part == 2)
		{
			tree.push_back(two("Potion", family(200), family(203) - 1, family(205), family(206) - 1));
			tree.push_back(one("Return Scroll", 203));
			tree.push_back(one("Weapon & Armor Scroll", 204));
			tree.push_back(two("Arrow, Throwing Stars, and Bullets", family(206), family(208) - 1, family(233),
							   family(234) - 1));
			tree.push_back(one("Summon", 210));
		}

		tree.push_back(heading("Etc", part == 3 ? 0 : 3));

		if (part == 3)
		{
			tree.push_back(two("Dropping", 4000000, 4003999, 0, -1));
			tree.push_back(two("Ore", 4004000, 4019999, 0, -1));
			tree.push_back(one("Game", 408));
			tree.push_back(one("Stimulator & How to Make", 413));
		}

		// v83 sets the set-up items' range twice over, keeping only
		// 3990000-3999999; this takes the chairs it meant as well.
		Category setup = two("Setup", family(301), family(400) - 1, 0, -1);
		setup.heading = true;
		tree.push_back(setup);

		return tree;
	}

	int16_t UIItemSearch::row_at(Point<int16_t> cursorpos) const
	{
		if (panel == Panel::NONE || hiding)
			return -1;

		Point<int16_t> pos = cursorpos - position - panel_offset();
		int16_t top = panel == Panel::POPULAR ? POPULAR_TOP : panel == Panel::RESULTS ? FOUND_TOP : TREE_TOP;
		int16_t step = panel == Panel::POPULAR ? POPULAR_STEP : panel == Panel::RESULTS ? FOUND_STEP : TREE_STEP;

		if (pos.x() < ROW_LEFT || pos.x() >= ROW_LEFT + ROW_WIDTH || pos.y() < top)
			return -1;

		auto row = static_cast<int16_t>((pos.y() - top) / step);

		return row < rows_shown() ? row : -1;
	}

	int16_t UIItemSearch::rows_shown() const
	{
		switch (panel)
		{
			case Panel::POPULAR:
				return static_cast<int16_t>(
						std::clamp<int32_t>(static_cast<int32_t>(popular.size()) - page * POPULAR_ROWS, 0, POPULAR_ROWS));
			case Panel::RESULTS:
				return static_cast<int16_t>(
						std::clamp<int32_t>(static_cast<int32_t>(found.size()) - page * FOUND_ROWS, 0, FOUND_ROWS));
			case Panel::CATEGORIES:
				return static_cast<int16_t>(names.size());
			default:
				return 0;
		}
	}

	int32_t UIItemSearch::item_at(int16_t row) const
	{
		if (panel == Panel::POPULAR)
			return popular[page * POPULAR_ROWS + row];

		return found[page * FOUND_ROWS + row];
	}

	Rectangle<int16_t> UIItemSearch::panel_bounds() const
	{
		Point<int16_t> origin = position + panel_offset();

		return Rectangle<int16_t>(origin, origin + panel_background.get_dimensions());
	}

	Rectangle<int16_t> UIItemSearch::order_bounds() const
	{
		Point<int16_t> origin = position + panel_offset();

		return Rectangle<int16_t>(origin + Point<int16_t>(ORDER.left(), ORDER.top()),
								  origin + Point<int16_t>(ORDER.right(), ORDER.bottom()));
	}

	Point<int16_t> UIItemSearch::panel_offset() const
	{
		return Point<int16_t>(slide, PANEL.y());
	}
}
