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

#include "../../Data/CashCatalog.h"
#include "../../Graphics/Text.h"

#include <optional>

namespace ms
{
	// A crush or friendship ring's order, which the BUY dialog sends once a
	// balance is picked (v83 46ffe7 and 470a5a).
	struct RingOrder
	{
		bool friendship;
		int32_t birthday;
		std::string to;
		std::string message;
	};

	// The Cash Shop's BUY dialog (UIClassic.nx CashShop.img/CSNotice/0): the
	// item, its price, and which balance pays for it. A commodity with sn 0
	// is a slot button's four slots. Only a balance that covers the price
	// can be checked, and for a ring, Maple Points can't pay.
	class UICashBuy : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::NOTICE;
		static constexpr bool FOCUSED = true;
		static constexpr bool TOGGLED = false;

		UICashBuy(const cash_catalog::Commodity& commodity, int32_t credit, int32_t points, int32_t prepaid, std::optional<RingOrder> ring = {});

		// Opens the dialog, after v83's checks: some balance must cover the
		// price, NX Prepaid alone pays for some items, an item the character
		// already has asks first, a slot button must find room in the
		// storage, and a bag a job advancement would widen past its last
		// slot asks first.
		static void open(const cash_catalog::Commodity& commodity, int32_t credit, int32_t points, int32_t prepaid);

		void draw(float inter) const override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		void buy();

		enum Buttons : uint16_t
		{
			BT_OK,
			BT_CANCEL
		};

		std::vector<Texture> parts;
		Texture checkboxes[2];
		Text name;
		Text price;
		Text question;
		int32_t sn;
		int32_t item_id;
		std::array<bool, cash_catalog::NUM_PAYMENTS> offered;
		size_t payment;
		std::optional<RingOrder> ring;
	};
}
