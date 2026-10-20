#include "UIPetItemSearch.h"

#include "UICharInfo.h"

#include "../KeyAction.h"
#include "../UI.h"

#include "../Components/MapleButton.h"

#include "../../Character/Pets.h"
#include "../../Data/ItemData.h"
#include "../../Graphics/GraphicsGL.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

#include <algorithm>
#include <utility>

namespace ms
{
	namespace
	{
		// The dialog (FUN_008fc1bf), against its top-left corner: dragged by
		// the strip above the text.
		const Point<int16_t> SEARCH = { 140, 76 };
		const Point<int16_t> CLOSE = { 195, 8 };
		constexpr int16_t DRAG_HEIGHT = 24;
		const Rectangle<int16_t> EDIT = Rectangle<int16_t>(15, 15 + 115, 78, 78 + 13);
		constexpr size_t EDIT_LIMIT = 256;
		// The results open under the dialog, drawn below it (FUN_008fb79b).
		const Point<int16_t> RESULTS = { 0, 120 };
		const Point<int16_t> SAVE = { 49, 94 };
		const Point<int16_t> DELETE = { 82, 94 };
		constexpr int16_t SLIDER_X = 142;
		constexpr Range<int16_t> SLIDER = Range<int16_t>(8, 8 + 75 - 13);
		// Five rows of names, each lit by a box behind it when selected or
		// under the cursor (FUN_008fba69).
		constexpr int16_t ROWS = 5, ROW_TOP = 12, ROW_STEP = 14;
		constexpr int16_t NAME_X = 10, FILL_X = 10, FILL_WIDTH = 138, FILL_HEIGHT = 13, HIT_WIDTH = 138;
		constexpr Color SELECTED = Color(0x5B7B9EFF);
		constexpr Color HOVERED = Color(0xBBC9D9FF);

		// How far down to draw Arial 12 whose top v83 puts at y: HC puts a
		// line's baseline a full line below where it's drawn.
		int16_t text_y(int16_t y)
		{
			static const int16_t line = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::WHITE, "|").height();

			return y + 11 - line;
		}

		// Every item's name without spaces, in String's order: equipment,
		// use, set-up, cash, pets and etc. (FUN_005d8796).
		const std::vector<std::pair<std::string, int32_t>> &item_names()
		{
			static std::vector<std::pair<std::string, int32_t>> names;

			if (!names.empty())
				return names;

			auto add = [&](nl::node items)
			{
				for (nl::node item : items)
					names.emplace_back(pets::search_key(item["name"].get_string()), std::stoi(item.name()));
			};

			for (nl::node category : nl::nx::string["Eqp.img"]["Eqp"])
				add(category);

			add(nl::nx::string["Consume.img"]);
			add(nl::nx::string["Ins.img"]);
			add(nl::nx::string["Cash.img"]);
			add(nl::nx::string["Pet.img"]);
			add(nl::nx::string["Etc.img"]["Etc"]);

			return names;
		}

		// Whether the search offers the item (FUN_005d3f1b, FUN_005d4a2b):
		// one that can be traded, and isn't for cash or a quest.
		bool offered(int32_t id)
		{
			if (!pets::searchable(id))
				return false;

			const ItemData &item = ItemData::get(id);

			if (!item.is_valid() || item.is_cashitem() || item.is_questitem() || item.get_properties().pquest)
				return false;

			return item.get_properties().trade_available || !item.is_untradable();
		}
	}

	UIPetItemSearch::UIPetItemSearch(Point<int16_t> pos) : UIElement(pos, Point<int16_t>())
	{
		nl::node ItemSearch = NxFiles::ui_classic["UIWindow.img"]["ExceptionItemSearch"];

		background = ItemSearch["backgrnd"];
		results_background = ItemSearch["resultback"];
		dimension = background.get_dimensions();

		buttons[Buttons::BT_SEARCH] = std::make_unique<MapleButton>(ItemSearch["BtSearch"], SEARCH);
		buttons[Buttons::BT_CLOSE] = std::make_unique<MapleButton>(NxFiles::ui_classic["Basic.img"]["BtClose"], CLOSE);
		buttons[Buttons::BT_SAVE] = std::make_unique<MapleButton>(ItemSearch["BtSave"], RESULTS + SAVE);
		buttons[Buttons::BT_DELETE] = std::make_unique<MapleButton>(ItemSearch["BtDelete"], RESULTS + DELETE);
		buttons[Buttons::BT_SAVE]->set_active(false);
		buttons[Buttons::BT_DELETE]->set_active(false);

		slider = Slider(Slider::Type::CLASSIC, SLIDER, SLIDER_X, ROWS, 0, [&](bool upwards)
		{
			offset += upwards ? -1 : 1;
		});

		edit = Textfield(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK,
						 Rectangle<int16_t>(EDIT.left(), EDIT.right(), text_y(EDIT.top()), EDIT.bottom()), EDIT_LIMIT);

		edit.set_enter_callback(
				[&](std::string text)
				{
					search(pets::trim(text));
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
	}

	void UIPetItemSearch::draw(float alpha) const
	{
		if (searched)
		{
			Point<int16_t> origin = position + RESULTS;
			results_background.draw(origin);

			GraphicsGL &graphics = GraphicsGL::get();

			for (int16_t r = 0; r < ROWS && offset + r < static_cast<int16_t>(names.size()); r++)
			{
				int16_t i = offset + r;
				int16_t top = ROW_TOP + ROW_STEP * r;

				if (i == selected || i == hovered)
				{
					Color fill = i == selected ? SELECTED : HOVERED;
					graphics.drawrectangle(origin.x() + FILL_X, origin.y() + top, FILL_WIDTH, FILL_HEIGHT, fill.r(),
										   fill.g(), fill.b(), fill.a());
				}

				names[i].draw(origin + Point<int16_t>(NAME_X, text_y(top)));
			}

			slider.draw(origin);
		}

		background.draw(position);
		edit.draw(position);

		UIElement::draw_buttons(alpha);
	}

	void UIPetItemSearch::update()
	{
		UIElement::update();

		edit.update(position);

		// The search goes with the exception window it adds to.
		auto charinfo = UI::get().get_element<UICharInfo>();

		if (!charinfo || !charinfo->is_active() || !charinfo->has_exceptions())
			close();
	}

	bool UIPetItemSearch::is_in_range(Point<int16_t> cursorpos) const
	{
		return Rectangle<int16_t>(position, position + dimension).contains(cursorpos)
			   || (searched && results_bounds().contains(cursorpos));
	}

	Cursor::State UIPetItemSearch::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		cursor = cursorpos;

		if (dragged)
		{
			if (clicked)
			{
				position = cursorpos - cursoroffset;

				return Cursor::State::CLICKING;
			}

			dragged = false;
		}

		bool released = !clicked && mouse_down;
		mouse_down = clicked;

		if (searched)
		{
			Cursor::State state = slider.send_cursor(cursorpos - position - RESULTS, clicked);

			if (state != Cursor::State::IDLE)
				return state;

			// A result under the cursor lights up and shows its tooltip, and
			// is selected as the button comes up on it (FUN_008fbd2e,
			// FUN_008fbde9).
			int16_t row = row_at(cursorpos);

			if (row != hovered)
			{
				hovered = row;

				if (hovered >= 0)
					UI::get().show_item(Tooltip::Parent::PETITEMSEARCH, results[hovered]);
				else
					UI::get().clear_tooltip(Tooltip::Parent::PETITEMSEARCH);
			}

			if (released && !is_holding_button() && results_bounds().contains(cursorpos))
				selected = hovered;
		}

		if (Cursor::State state = edit.send_cursor(cursorpos, clicked); state != Cursor::State::IDLE)
			return state;

		Cursor::State state = UIElement::send_cursor(clicked, cursorpos);
		Point<int16_t> pos = cursorpos - position;

		if (clicked && !is_holding_button() && pos.x() >= 0 && pos.x() < dimension.x() && pos.y() >= 0
			&& pos.y() < DRAG_HEIGHT)
		{
			dragged = true;
			cursoroffset = pos;
		}

		return state;
	}

	void UIPetItemSearch::remove_cursor()
	{
		UIElement::remove_cursor();

		slider.remove_cursor();
		mouse_down = false;

		if (hovered >= 0)
		{
			hovered = -1;
			UI::get().clear_tooltip(Tooltip::Parent::PETITEMSEARCH);
		}
	}

	void UIPetItemSearch::send_scroll(double yoffset)
	{
		if (searched && results_bounds().contains(cursor))
			slider.send_scroll(yoffset);
	}

	void UIPetItemSearch::send_key(int32_t, bool pressed, bool escape)
	{
		if (pressed && escape)
			close();
	}

	UIElement::Type UIPetItemSearch::get_type() const
	{
		return TYPE;
	}

	void UIPetItemSearch::close()
	{
		if (!is_active())
			return;

		edit.set_state(Textfield::State::DISABLED);
		UI::get().clear_tooltip(Tooltip::Parent::PETITEMSEARCH);
		deactivate();
	}

	Button::State UIPetItemSearch::button_pressed(uint16_t buttonid)
	{
		switch (buttonid)
		{
			case Buttons::BT_SEARCH:
				search(pets::trim(edit.get_text()));
				break;
			case Buttons::BT_CLOSE:
				close();
				break;
			case Buttons::BT_SAVE:
			case Buttons::BT_DELETE:
				if (selected >= 0 && selected < static_cast<int16_t>(results.size()))
					if (auto charinfo = UI::get().get_element<UICharInfo>())
					{
						if (buttonid == Buttons::BT_SAVE)
							charinfo->add_exception(results[selected]);
						else
							charinfo->remove_exception(results[selected]);
					}

				break;
			default:
				break;
		}

		return Button::State::NORMAL;
	}

	void UIPetItemSearch::search(const std::string &text)
	{
		// Each search opens the results anew (FUN_008fbe4b): every item
		// tradeable by name in String's order, or none for no text.
		results.clear();
		names.clear();

		std::string key = pets::search_key(text);

		if (!key.empty())
			for (const auto &[name, id] : item_names())
				if (name.find(key) != std::string::npos && offered(id))
					results.push_back(id);

		for (int32_t id : results)
			names.emplace_back(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK,
							   ItemData::get(id).get_name());

		searched = true;
		offset = 0;
		hovered = -1;
		selected = -1;
		slider.setrows(0, ROWS, static_cast<int16_t>(results.size()));
		buttons[Buttons::BT_SAVE]->set_active(true);
		buttons[Buttons::BT_DELETE]->set_active(true);
		UI::get().clear_tooltip(Tooltip::Parent::PETITEMSEARCH);
	}

	int16_t UIPetItemSearch::row_at(Point<int16_t> cursorpos) const
	{
		Point<int16_t> pos = cursorpos - position - RESULTS;

		for (int16_t r = 0; r < ROWS && offset + r < static_cast<int16_t>(results.size()); r++)
		{
			int16_t top = ROW_TOP + ROW_STEP * r;

			if (Rectangle<int16_t>(0, HIT_WIDTH, top, top + ROW_STEP).contains(pos))
				return offset + r;
		}

		return -1;
	}

	Rectangle<int16_t> UIPetItemSearch::results_bounds() const
	{
		Point<int16_t> origin = position + RESULTS;

		return Rectangle<int16_t>(origin, origin + results_background.get_dimensions());
	}
}
