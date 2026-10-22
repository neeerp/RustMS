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
#include "UICouponDialog.h"
#include "UICashShop.h"
#include "UINotice.h"

#include "../KeyAction.h"
#include "../UI.h"

#include "../Components/MapleButton.h"

#include "../../Audio/Audio.h"
#include "../../Constants.h"
#include "../../Data/CashCoupon.h"
#include "../../Util/NxFiles.h"

#include "../../Net/Packets/GameplayPackets.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// The white box of the art, raised by the space above the glyphs
		// (v83 puts the edit at 17,53, 230 wide).
		const Rectangle<int16_t> CODE_BOX = Rectangle<int16_t>(17, 247, 53, 68);
	}

	UICouponDialog::UICouponDialog() : UIElement(Point<int16_t>(0, 0), Point<int16_t>(0, 0))
	{
		background = NxFiles::ui_classic["UIWindow.img"]["Coupon"]["backgrnd"];
		dimension = background.get_dimensions();
		position = Point<int16_t>((Constants::Constants::get().get_viewwidth() - dimension.x()) / 2, (Constants::Constants::get().get_viewheight() - dimension.y()) / 2);

		nl::node Basic = NxFiles::ui_classic["Basic.img"];

		buttons[Buttons::BT_OK] = std::make_unique<MapleButton>(Basic["BtOK2"], Point<int16_t>(159, 95));
		buttons[Buttons::BT_CANCEL] = std::make_unique<MapleButton>(Basic["BtCancel2"], Point<int16_t>(209, 95));

		code = Textfield(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, CODE_BOX, cash_coupon::CODE_LIMIT);
		code.set_enter_callback([&](std::string) { confirm(); });
		code.set_key_callback(KeyAction::Id::ESCAPE, [&]() { close(); });
		code.set_state(Textfield::State::FOCUSED);

		Sound(Sound::Name::DLGNOTICE).play();
	}

	void UICouponDialog::draw(float inter) const
	{
		background.draw(position);
		code.draw(position);

		UIElement::draw(inter);
	}

	void UICouponDialog::update()
	{
		UIElement::update();

		// v83 drops a "-" typed at the end, as the art asks.
		std::string text = code.get_text();
		std::string kept = cash_coupon::without_hyphen(text);

		if (kept != text)
			code.change_text(kept);

		code.update(position);
	}

	Cursor::State UICouponDialog::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		Cursor::State state = code.send_cursor(cursorpos, clicked);

		if (state != Cursor::State::IDLE)
			return state;

		return UIElement::send_cursor(clicked, cursorpos);
	}

	void UICouponDialog::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (!pressed)
			return;

		if (escape)
			close();
		else if (keycode == KeyAction::Id::RETURN)
			confirm();
	}

	UIElement::Type UICouponDialog::get_type() const
	{
		return TYPE;
	}

	Button::State UICouponDialog::button_pressed(uint16_t buttonid)
	{
		if (buttonid == Buttons::BT_OK)
			confirm();
		else
			close();

		return Button::State::NORMAL;
	}

	void UICouponDialog::confirm()
	{
		std::string text = cash_coupon::trim(code.get_text());

		close();

		if (text.empty())
		{
			UI::get().emplace<UIOk>(cash_coupon::NO_CODE, [](bool) {});

			return;
		}

		if (auto cashshop = UI::get().get_element<UICashShop>(); cashshop && cashshop->request())
			CouponCodePacket(text).dispatch();
	}

	void UICouponDialog::close()
	{
		code.set_state(Textfield::State::DISABLED);

		deactivate();
	}
}
