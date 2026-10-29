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
#include "UICashBuy.h"
#include "UICashShop.h"
#include "UINotice.h"

#include "../KeyAction.h"
#include "../UI.h"

#include "../Components/MapleButton.h"

#include "../../Constants.h"
#include "../../Character/Marriage.h"
#include "../../Data/CurseData.h"
#include "../../Data/ItemData.h"
#include "../../Graphics/Geometry.h"
#include "../../Gameplay/Stage.h"
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
		// The ring and present dialog (v83 7e4cf7) is CSNotice/2, whose
		// boxes sit 27 pixels higher.
		Rectangle<int16_t> on_notice(Rectangle<int16_t> box)
		{
			box.shift(Point<int16_t>(0, -27));

			return box;
		}
		// CSNotice/2 has "Enter your message here," in its message box,
		// which v83 (7e5467) covers while a field has the keys or the
		// message has text.
		const Point<int16_t> HINT_AT = Point<int16_t>(64, 48);
		const ColorBox HINT_COVER = ColorBox(138, 14, Color::Name::WHITE, 1.0f);
		// The client keeps names to 12 characters and messages to 73.
		constexpr size_t NAME_LIMIT = 12;
		constexpr size_t MESSAGE_LIMIT = 73;
		// StringPool 0x217.
		constexpr const char* NOT_ENOUGH_CASH = "You don't have enough cash.";
		// OK and Cancel, from the right edge and the bottom of the art.
		const Point<int16_t> OK_FROM_CORNER = Point<int16_t>(123, 31);
		const Point<int16_t> CANCEL_FROM_CORNER = Point<int16_t>(73, 31);
	}

	UICashGift::UICashGift(const cash_catalog::Commodity& commodity) : UIElement(Point<int16_t>(0, 0), Point<int16_t>(0, 0)), asking_birthday(false), sn(commodity.sn), credit(0), points(0), prepaid(0), preset(false)
	{
		background = NxFiles::ui_classic["CashShop.img"]["CSGift"]["backgrnd"];

		to = Textfield(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, TO_BOX, NAME_LIMIT);
		message = Textfield(Text::Font::A11M, Text::Alignment::LEFT, Color::Name::BLACK, MESSAGE_BOX, MESSAGE_LIMIT);
		message.set_wrap(MESSAGE_BOX.width());
		birthday = Textfield(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, BIRTHDAY_BOX, cash_catalog::BIRTHDAY_LIMIT);

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

	UICashGift::UICashGift(const cash_catalog::Commodity& commodity, int32_t credit, int32_t points, int32_t prepaid) : UICashGift(commodity)
	{
		ring = commodity;
		this->credit = credit;
		this->points = points;
		this->prepaid = prepaid;

		to.set_state(Textfield::State::DISABLED);

		asking_birthday = true;
		background = NxFiles::ui_classic["CashShop.img"]["CSNotice"]["1"]["backgrnd"];
		place();

		birthday.set_state(Textfield::State::FOCUSED);
	}

	UICashGift::UICashGift(const cash_catalog::Commodity& commodity, int32_t prepaid, const std::string& to) : UICashGift(commodity, 0, 0, prepaid)
	{
		// The name is shown but can't be changed (v83 7e5c77).
		preset = true;
		this->to.change_text(to);
		this->to.set_shown_disabled();
		message.set_key_callback(KeyAction::Id::TAB, []() {});
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
			bool typing = to.get_state() == Textfield::State::FOCUSED || message.get_state() == Textfield::State::FOCUSED;

			if (ring && (typing || !message.empty()))
				HINT_COVER.draw(position + HINT_AT);

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
		if (ring)
			return confirm_ring();

		if (asking_birthday)
		{
			// The server answers a wrong code with its own notice, and a
			// right one with the receiver, whose answer asks the rest
			// (GIFT_INFO_RESULT). v83 (FUN_007e47ef) stars out curse words
			// in both lines first.
			int32_t code = cash_catalog::birthday_code(birthday.get_text());
			std::string receiver = CurseData::filter(to.get_text());
			std::string note = CurseData::filter(message.get_text());

			close();

			if (auto cashshop = UI::get().get_element<UICashShop>(); cashshop && cashshop->request())
				GiftCashItemPacket(code, sn, receiver, note).dispatch();

			return;
		}

		if (auto [refusal, name] = plain_refusal(to.get_text(), message.get_text()); refusal)
		{
			UI::get().emplace<UIOk>(refusal, [name = name](bool)
			{
				if (auto gift = UI::get().get_element<UICashGift>())
					gift->refocus(name);
			});

			return;
		}

		to.set_state(Textfield::State::DISABLED);
		message.set_state(Textfield::State::DISABLED);

		asking_birthday = true;
		background = NxFiles::ui_classic["CashShop.img"]["CSNotice"]["1"]["backgrnd"];
		place();

		birthday.set_state(Textfield::State::FOCUSED);
	}

	std::pair<const char*, bool> UICashGift::gift_refusal(const std::string& name, const std::string& note)
	{
		if (name.size() < 4 || name.size() > NAME_LIMIT)
			return { "Please enter the name of the receiving character.", true };

		if (name == Stage::get().get_player().get_name())
			return { "You cannot send gifts to yourself.", true };

		if (!CurseData::valid(name))
			return { "Cursing is not allowed.", true };

		if (!CurseData::valid(note))
			return { "Cursing is not allowed.", false };

		if (note.empty())
			return { "Please enter the message.", false };

		if (note.size() > 72)
			return { "The message can be no longer than 72 characters long.", false };

		return { nullptr, false };
	}

	std::pair<const char*, bool> UICashGift::plain_refusal(const std::string& name, const std::string& note)
	{
		// v83 reads several names split by ';' and checks each but the last
		// like a ring's; only one is sent here, so that one isn't checked.
		if (name.find_first_not_of(" ;") == std::string::npos)
			return { "Please enter the name of the receiving character.", true };

		if (!CurseData::valid(note))
			return { "Cursing is not allowed.", false };

		if (note.empty())
			return { "Please enter the message.", false };

		if (note.size() > 72)
			return { "The message can be no longer than 72 characters long.", false };

		return { nullptr, false };
	}

	void UICashGift::refocus(bool name)
	{
		(name && !preset ? to : message).set_state(Textfield::State::FOCUSED);
	}

	void UICashGift::confirm_ring()
	{
		if (asking_birthday)
		{
			// v83 (0x470078) checks the price next: NX Credit or NX Prepaid
			// must cover it. A present has only NX Prepaid (472e6c).
			if (credit < ring->price && prepaid < ring->price)
			{
				close();
				UI::get().emplace<UIOk>(NOT_ENOUGH_CASH, [](bool) {});

				return;
			}

			birthday.set_state(Textfield::State::DISABLED);

			asking_birthday = false;
			background = NxFiles::ui_classic["CashShop.img"]["CSNotice"]["2"]["backgrnd"];
			to.set_bounds(on_notice(TO_BOX));
			message.set_bounds(on_notice(MESSAGE_BOX));
			place();

			(preset ? message : to).set_state(Textfield::State::FOCUSED);

			return;
		}

		// v83 7e5863 checks the name, then the message, and each refusal
		// leaves the dialog open.
		if (auto [refusal, name] = gift_refusal(to.get_text(), message.get_text()); refusal)
		{
			UI::get().emplace<UIOk>(refusal, [name = name](bool)
			{
				if (auto gift = UI::get().get_element<UICashGift>())
					gift->refocus(name);
			});

			return;
		}

		// StringPool 0x200, then the BUY dialog. Opening the question
		// replaces this dialog, so everything is copied first.
		cash_catalog::Commodity commodity = *ring;

		if (preset)
		{
			std::string question = ItemData::get(commodity.item_id).get_name() + " (" + std::to_string(commodity.price) +
								   " NX)\\nWill you buy this item as a gift for\\n'" + to.get_text() + "'?";
			// A package goes at once; an item is looked up first, like the
			// Gift button's (v83 472c67).
			auto send = [code = cash_catalog::birthday_code(birthday.get_text()), sn = commodity.sn, receiver = to.get_text(),
						 note = CurseData::filter(message.get_text()), mode = static_cast<uint8_t>(commodity.item_id / 10000 == 910 ? 0x1F : 4)]()
			{
				if (auto cashshop = UI::get().get_element<UICashShop>(); cashshop && cashshop->request())
					GiftCashItemPacket(code, sn, receiver, note, mode).dispatch();
			};

			close();
			UI::get().emplace<UIYesNo>(question, [send](bool yes) { if (yes) send(); });

			return;
		}

		RingOrder order = { marriage::ring_kind(commodity.item_id) == RingKind::FRIENDSHIP, cash_catalog::birthday_code(birthday.get_text()),
							CurseData::filter(to.get_text()), CurseData::filter(message.get_text()) };
		int32_t balances[3] = { credit, points, prepaid };
		std::string question = ItemData::get(commodity.item_id).get_name() + " (" + std::to_string(commodity.price) +
							   " NX)\\nWill you buy this item as a gift for\\n'" + order.to + "'?";

		close();

		UI::get().emplace<UIYesNo>(question, [commodity, order, balances](bool yes)
		{
			if (yes)
				UI::get().emplace<UICashBuy>(commodity, balances[0], balances[1], balances[2], order);
		});
	}

	void UICashGift::close()
	{
		for (Textfield* field : { &to, &message, &birthday })
			field->set_state(Textfield::State::DISABLED);

		deactivate();
	}
}
