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
#pragma once

#include "../UIElement.h"

#include "../Components/Textfield.h"

#include <utility>
#include <vector>

namespace ms
{
	// The Cash Shop's SEARCH ITEM dialog (UIClassic.nx
	// CashShop.img/CSItemSearch/PopUp, v83 4b5ebf): a name and a price
	// range, looked for here in the whole catalog, with no packet.
	class UICashSearch : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::NOTICE;
		static constexpr bool FOCUSED = true;
		static constexpr bool TOGGLED = false;

		UICashSearch();

		void draw(float inter) const override;

		void update() override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		void confirm();

		void close();

		enum Buttons : uint16_t
		{
			BT_OK,
			BT_CANCEL,
			BT_RANGE
		};

		Texture background;
		Textfield name;
		// The ranges after the combo's first, empty entry.
		std::vector<std::pair<int32_t, int32_t>> ranges;
	};
}
