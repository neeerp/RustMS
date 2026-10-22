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

#include "../../Data/CashCatalog.h"

#include <optional>

namespace ms
{
	// The Cash Shop's GIFT dialog: the receiver and a message
	// (UIClassic.nx CashShop.img/CSGift), then the birthday code that
	// confirms the purchase (CSNotice/1). A crush or friendship ring asks
	// for the birthday code first, then the partner and a message, and is
	// paid for in the BUY dialog (v83 46ffe7 and 470a5a).
	class UICashGift : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::NOTICE;
		static constexpr bool FOCUSED = true;
		static constexpr bool TOGGLED = false;

		UICashGift(const cash_catalog::Commodity& commodity);
		// A ring pair for the player and a partner, with the balances.
		UICashGift(const cash_catalog::Commodity& commodity, int32_t credit, int32_t points, int32_t prepaid);

		void draw(float inter) const override;

		void update() override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		// Center the dialog and put OK and Cancel at the bottom of its art.
		void place();

		void confirm();

		// A ring's steps after each page: the price check, then the
		// question and the BUY dialog.
		void confirm_ring();

		void close();

		enum Buttons : uint16_t
		{
			BT_OK,
			BT_CANCEL
		};

		Texture background;
		Textfield to;
		Textfield message;
		Textfield birthday;
		bool asking_birthday;
		int32_t sn;
		std::optional<cash_catalog::Commodity> ring;
		int32_t credit;
		int32_t points;
		int32_t prepaid;
	};
}
