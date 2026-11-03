//////////////////////////////////////////////////////////////////////////////////
//	This file is part of the continued Journey MMORPG client					//
//	Copyright (C) 2015-2019  Daniel Allendorf, Ryan Payton						//
//																				//
//	This program is free software: you can redistribute it and/or modify		//
//	it under the terms of the GNU Affero General Public License as published by	//
//	the Free Software Foundation, either version 3 of the License, or			//
//	(at your option) any later version.											//
//																				//
//	This program is distributed in the hope that it will be useful,				//
//	but WITHOUT ANY WARRANTY; without even the implied warranty of				//
//	MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the				//
//	GNU Affero General Public License for more details.							//
//																				//
//	You should have received a copy of the GNU Affero General Public License	//
//	along with this program.  If not, see <https://www.gnu.org/licenses/>.		//
//////////////////////////////////////////////////////////////////////////////////
#include "UICashSearch.h"
#include "UICashShop.h"

#include "../KeyAction.h"
#include "../UI.h"

#include "../Components/MapleButton.h"
#include "../Components/MapleComboBox.h"

#include "../../Constants.h"
#include "../../Data/CashCatalog.h"
#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

#include <cstdint>

namespace ms
{
	namespace
	{
		// The edit and the combo, both 140 x 16 (4b5f27); the combo's caps
		// are 5 and 18 wide.
		const Rectangle<int16_t> NAME_BOX = Rectangle<int16_t>(103, 243, 27, 43);
		constexpr Point<int16_t> RANGE_AT = Point<int16_t>(103, 52);
		constexpr int64_t RANGE_MIDDLE = 117;
		constexpr Point<int16_t> OK_AT = Point<int16_t>(58, 104);
		constexpr Point<int16_t> CANCEL_AT = Point<int16_t>(159, 104);
	}

	UICashSearch::UICashSearch() : UIElement(Point<int16_t>(0, 0), Point<int16_t>(0, 0))
	{
		nl::node CSItemSearch = NxFiles::ui_classic["CashShop.img"]["CSItemSearch"];

		background = CSItemSearch["PopUp"]["backgrnd"];
		dimension = background.get_dimensions();
		position = Point<int16_t>((Constants::Constants::get().get_viewwidth() - dimension.x()) / 2, (Constants::Constants::get().get_viewheight() - dimension.y()) / 2);

		nl::node Basic = NxFiles::ui_classic["Basic.img"];

		buttons[Buttons::BT_OK] = std::make_unique<MapleButton>(Basic["BtOK2"], OK_AT);
		buttons[Buttons::BT_CANCEL] = std::make_unique<MapleButton>(Basic["BtCancel2"], CANCEL_AT);

		// An empty entry for any price, then "min ~ max" for each of
		// CSItemSearch/Price (4b5f27).
		std::vector<std::string> options = { "" };

		for (nl::node range : CSItemSearch["Price"])
		{
			int32_t min = static_cast<int32_t>(range["0"].get_integer());
			int32_t max = static_cast<int32_t>(range["1"].get_integer());

			ranges.emplace_back(min, max);
			options.push_back(std::to_string(min) + " ~ " + std::to_string(max));
		}

		buttons[Buttons::BT_RANGE] = std::make_unique<MapleComboBox>(MapleComboBox::Type::DEFAULT, options, 0, position, RANGE_AT, RANGE_MIDDLE);

		name = Textfield(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, NAME_BOX, UINT16_MAX);
		name.set_enter_callback([&](std::string) { confirm(); });
		name.set_key_callback(KeyAction::Id::ESCAPE, [&]() { close(); });
		name.set_state(Textfield::State::FOCUSED);
	}

	void UICashSearch::draw(float inter) const
	{
		background.draw(position);
		name.draw(position);

		UIElement::draw(inter);
	}

	void UICashSearch::update()
	{
		UIElement::update();

		name.update(position);
	}

	Cursor::State UICashSearch::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		// The open list covers what is under it.
		auto& range = buttons[Buttons::BT_RANGE];

		if (range->is_pressed() && range->in_combobox(cursorpos))
		{
			Cursor::State state = range->send_cursor(clicked, cursorpos);

			return clicked && state == Cursor::State::IDLE ? Cursor::State::CLICKING : state;
		}

		Cursor::State state = name.send_cursor(cursorpos, clicked);

		if (state != Cursor::State::IDLE)
			return state;

		return UIElement::send_cursor(clicked, cursorpos);
	}

	void UICashSearch::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (!pressed)
			return;

		if (escape)
			close();
		else if (keycode == KeyAction::Id::RETURN)
			confirm();
	}

	UIElement::Type UICashSearch::get_type() const
	{
		return TYPE;
	}

	Button::State UICashSearch::button_pressed(uint16_t buttonid)
	{
		switch (buttonid)
		{
		case Buttons::BT_RANGE:
			buttons[buttonid]->toggle_pressed();
			break;
		case Buttons::BT_OK:
			confirm();
			break;
		default:
			close();
			break;
		}

		return Button::State::NORMAL;
	}

	void UICashSearch::confirm()
	{
		// The empty entry takes any price (4b6270).
		uint16_t chosen = buttons[Buttons::BT_RANGE]->get_selected();
		auto [min, max] = chosen > 0 && chosen <= ranges.size() ? ranges[chosen - 1] : std::pair<int32_t, int32_t>(0, INT32_MAX);
		std::string query = cash_catalog::search_query(name.get_text());

		close();

		if (auto cashshop = UI::get().get_element<UICashShop>())
			cashshop->show_search(query, min, max);
	}

	void UICashSearch::close()
	{
		name.set_state(Textfield::State::DISABLED);

		deactivate();
	}
}
