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
#include "UICashGift.h"
#include "UICashShop.h"

#include "../KeyAction.h"
#include "../UI.h"

#include "../Components/MapleButton.h"

#include "../../Constants.h"
#include "../../Data/CurseData.h"
#include "../../Util/NxFiles.h"

#include "../../Net/Packets/GameplayPackets.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// The fields' white boxes, measured on CSGift/backgrnd and
		// CSNotice/1, raised by the space above the text's glyphs. The
		// message box is ruled into two rows of 15 pixels.
		const Rectangle<int16_t> TO_BOX = Rectangle<int16_t>(84, 241, 45, 64);
		const Rectangle<int16_t> MESSAGE_BOX = Rectangle<int16_t>(26, 240, 71, 104);
		const Rectangle<int16_t> BIRTHDAY_BOX = Rectangle<int16_t>(22, 242, 48, 67);
		// The client keeps names to 12 characters and messages to 73.
		constexpr size_t NAME_LIMIT = 12;
		constexpr size_t MESSAGE_LIMIT = 73;
		constexpr size_t BIRTHDAY_LIMIT = 8;
		// OK and Cancel, from the right edge and the bottom of the art.
		const Point<int16_t> OK_FROM_CORNER = Point<int16_t>(123, 31);
		const Point<int16_t> CANCEL_FROM_CORNER = Point<int16_t>(73, 31);

		// The birthday code as the number YYYYMMDD, or 0 if it isn't one.
		int32_t birthday_code(const std::string& text)
		{
			if (text.size() != BIRTHDAY_LIMIT)
				return 0;

			for (char c : text)
				if (c < '0' || c > '9')
					return 0;

			return std::stoi(text);
		}
	}

	UICashGift::UICashGift(const cash_catalog::Commodity& commodity) : UIElement(Point<int16_t>(0, 0), Point<int16_t>(0, 0)), asking_birthday(false), sn(commodity.sn)
	{
		background = NxFiles::ui_classic["CashShop.img"]["CSGift"]["backgrnd"];

		to = Textfield(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, TO_BOX, NAME_LIMIT);
		message = Textfield(Text::Font::A11M, Text::Alignment::LEFT, Color::Name::BLACK, MESSAGE_BOX, MESSAGE_LIMIT);
		message.set_wrap(MESSAGE_BOX.width());
		birthday = Textfield(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, BIRTHDAY_BOX, BIRTHDAY_LIMIT);

		to.set_enter_callback([&](std::string) { message.set_state(Textfield::State::FOCUSED); });
		to.set_key_callback(KeyAction::Id::TAB, [&]() { message.set_state(Textfield::State::FOCUSED); });
		message.set_enter_callback([&](std::string) { confirm(); });
		birthday.set_enter_callback([&](std::string) { confirm(); });
		message.set_key_callback(KeyAction::Id::TAB, [&]() { to.set_state(Textfield::State::FOCUSED); });

		for (Textfield* field : { &to, &message, &birthday })
			field->set_key_callback(KeyAction::Id::ESCAPE, [&]() { close(); });

		place();

		to.set_state(Textfield::State::FOCUSED);
	}

	void UICashGift::place()
	{
		dimension = background.get_dimensions();
		position = Point<int16_t>((Constants::Constants::get().get_viewwidth() - dimension.x()) / 2, (Constants::Constants::get().get_viewheight() - dimension.y()) / 2);

		nl::node Basic = NxFiles::ui_classic["Basic.img"];

		buttons[Buttons::BT_OK] = std::make_unique<MapleButton>(Basic["BtOK2"], dimension - OK_FROM_CORNER);
		buttons[Buttons::BT_CANCEL] = std::make_unique<MapleButton>(Basic["BtCancel2"], dimension - CANCEL_FROM_CORNER);
	}

	void UICashGift::draw(float inter) const
	{
		background.draw(position);

		if (asking_birthday)
		{
			birthday.draw(position);
		}
		else
		{
			to.draw(position);
			message.draw(position);
		}

		UIElement::draw(inter);
	}

	void UICashGift::update()
	{
		UIElement::update();

		if (asking_birthday)
		{
			birthday.update(position);
		}
		else
		{
			to.update(position);
			message.update(position);
		}
	}

	Cursor::State UICashGift::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		if (asking_birthday)
		{
			Cursor::State state = birthday.send_cursor(cursorpos, clicked);

			if (state != Cursor::State::IDLE)
				return state;
		}
		else
		{
			for (Textfield* field : { &to, &message })
			{
				Cursor::State state = field->send_cursor(cursorpos, clicked);

				if (state != Cursor::State::IDLE)
					return state;
			}
		}

		return UIElement::send_cursor(clicked, cursorpos);
	}

	void UICashGift::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (!pressed)
			return;

		if (escape)
			close();
		else if (keycode == KeyAction::Id::RETURN)
			confirm();
	}

	UIElement::Type UICashGift::get_type() const
	{
		return TYPE;
	}

	Button::State UICashGift::button_pressed(uint16_t buttonid)
	{
		if (buttonid == Buttons::BT_OK)
			confirm();
		else
			close();

		return Button::State::NORMAL;
	}

	void UICashGift::confirm()
	{
		if (asking_birthday)
		{
			// The server answers a wrong code with its own notice. v83
			// (FUN_007e47ef) stars out curse words in both lines first.
			if (auto cashshop = UI::get().get_element<UICashShop>(); cashshop && cashshop->request())
				GiftCashItemPacket(birthday_code(birthday.get_text()), sn, CurseData::filter(to.get_text()),
								   CurseData::filter(message.get_text())).dispatch();

			close();

			return;
		}

		// Both a receiver and a message are needed.
		if (to.empty())
		{
			to.set_state(Textfield::State::FOCUSED);

			return;
		}

		if (message.empty())
		{
			message.set_state(Textfield::State::FOCUSED);

			return;
		}

		to.set_state(Textfield::State::DISABLED);
		message.set_state(Textfield::State::DISABLED);

		asking_birthday = true;
		background = NxFiles::ui_classic["CashShop.img"]["CSNotice"]["1"]["backgrnd"];
		place();

		birthday.set_state(Textfield::State::FOCUSED);
	}

	void UICashGift::close()
	{
		for (Textfield* field : { &to, &message, &birthday })
			field->set_state(Textfield::State::DISABLED);

		deactivate();
	}
}
