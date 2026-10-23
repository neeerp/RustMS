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
#include "UINameChange.h"
#include "UICashShop.h"
#include "UINotice.h"

#include "../KeyAction.h"
#include "../UI.h"

#include "../Components/MapleButton.h"

#include "../../Audio/Audio.h"
#include "../../Configuration.h"
#include "../../Constants.h"
#include "../../Data/CashCatalog.h"
#include "../../Data/CurseData.h"
#include "../../Data/NameChange.h"
#include "../../Gameplay/Stage.h"
#include "../../Util/CharacterName.h"
#include "../../Util/NxFiles.h"

#include "../../Net/Packets/CharCreationPackets.h"
#include "../../Net/Packets/GameplayPackets.h"

#include <nlnx/nx.hpp>

#include <algorithm>
#include <cstdio>

namespace ms
{
	namespace
	{
		// CSNotice/1's white box, as the gift dialog measures it.
		const Rectangle<int16_t> BIRTHDAY_BOX = Rectangle<int16_t>(22, 242, 48, 67);
		// OK and Cancel on CSNotice/1, from the right edge and the bottom.
		const Point<int16_t> OK_FROM_CORNER = Point<int16_t>(123, 31);
		const Point<int16_t> CANCEL_FROM_CORNER = Point<int16_t>(73, 31);

		// The rules sheet's text and scroll bar (v83 007ee4d8).
		const Point<int16_t> RULES_TEXT_AT = Point<int16_t>(17, 47);
		constexpr uint16_t RULES_WIDTH = 350;
		constexpr int16_t RULES_VIEW = 317;
		constexpr int16_t RULES_STEP = 7;
		constexpr int16_t RULES_SLIDER_X = 384;
		constexpr int16_t RULES_SLIDER_HEIGHT = 324;
		// The slider's rows must stay a few pixels high to show its thumb, so
		// one row may hold several steps.
		constexpr int16_t RULES_SLIDER_ROWS = 100;

		// The name dialog's edit (v83 007edc1e puts it at 26,51, 212 wide).
		const Rectangle<int16_t> NAME_BOX = Rectangle<int16_t>(26, 238, 51, 66);

		// v83 draws these windows from their art's corner, whatever its
		// origin says.
		void draw_from_corner(const Texture& texture, Point<int16_t> position)
		{
			texture.draw(position + texture.get_origin());
		}

		Point<int16_t> centered(Point<int16_t> dimension)
		{
			return Point<int16_t>((Constants::Constants::get().get_viewwidth() - dimension.x()) / 2,
								  (Constants::Constants::get().get_viewheight() - dimension.y()) / 2);
		}

		std::string rules_text()
		{
			nl::node sheet = nl::nx::string["NameChange.img"]["NameChange"];
			std::string text;

			for (int i = 0; i <= 30; i++)
			{
				char name[8];
				std::snprintf(name, sizeof(name), "Text%02d", i);
				text += sheet[name].get_string();
			}

			return name_change::rules_text(text);
		}
	}

	UINameChangeBirthday::UINameChangeBirthday(int32_t sn, int32_t price, int32_t prepaid) : UIElement(Point<int16_t>(0, 0), Point<int16_t>(0, 0)), sn(sn), price(price), prepaid(prepaid)
	{
		background = NxFiles::ui_classic["CashShop.img"]["CSNotice"]["1"]["backgrnd"];
		dimension = background.get_dimensions();
		position = centered(dimension);

		nl::node Basic = NxFiles::ui_classic["Basic.img"];

		buttons[Buttons::BT_OK] = std::make_unique<MapleButton>(Basic["BtOK2"], dimension - OK_FROM_CORNER);
		buttons[Buttons::BT_CANCEL] = std::make_unique<MapleButton>(Basic["BtCancel2"], dimension - CANCEL_FROM_CORNER);

		birthday = Textfield(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, BIRTHDAY_BOX, cash_catalog::BIRTHDAY_LIMIT);
		birthday.set_enter_callback([&](std::string) { confirm(); });
		birthday.set_key_callback(KeyAction::Id::ESCAPE, [&]() { close(); });
		birthday.set_state(Textfield::State::FOCUSED);
	}

	void UINameChangeBirthday::draw(float inter) const
	{
		background.draw(position);
		birthday.draw(position);

		UIElement::draw(inter);
	}

	void UINameChangeBirthday::update()
	{
		UIElement::update();

		birthday.update(position);
	}

	Cursor::State UINameChangeBirthday::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		Cursor::State state = birthday.send_cursor(cursorpos, clicked);

		if (state != Cursor::State::IDLE)
			return state;

		return UIElement::send_cursor(clicked, cursorpos);
	}

	void UINameChangeBirthday::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (!pressed)
			return;

		if (escape)
			close();
		else if (keycode == KeyAction::Id::RETURN)
			confirm();
	}

	UIElement::Type UINameChangeBirthday::get_type() const
	{
		return TYPE;
	}

	Button::State UINameChangeBirthday::button_pressed(uint16_t buttonid)
	{
		if (buttonid == Buttons::BT_OK)
			confirm();
		else
			close();

		return Button::State::NORMAL;
	}

	void UINameChangeBirthday::confirm()
	{
		int32_t code = cash_catalog::birthday_code(birthday.get_text());

		close();

		// v83 470323 pays only with NX Prepaid. The server answers a wrong
		// code.
		if (prepaid < price)
		{
			UI::get().emplace<UIOk>(name_change::NO_PREPAID, [](bool) {});

			return;
		}

		if (auto cashshop = UI::get().get_element<UICashShop>(); cashshop && cashshop->request())
		{
			cashshop->set_name_change(sn);
			NameChangeRulesPacket(Stage::get().get_player().get_oid(), code).dispatch();
		}
	}

	void UINameChangeBirthday::close()
	{
		birthday.set_state(Textfield::State::DISABLED);

		deactivate();
	}

	UINameChangeRules::UINameChangeRules(int32_t sn) : UIElement(Point<int16_t>(0, 0), Point<int16_t>(0, 0)), step(0), sn(sn)
	{
		background = NxFiles::ui_classic["CashShop.img"]["CSChangeName"]["Base"]["backgrndnotice"];
		dimension = background.get_dimensions();
		position = centered(dimension);

		nl::node Basic = NxFiles::ui_classic["Basic.img"];

		buttons[Buttons::BT_OK] = std::make_unique<MapleButton>(Basic["BtOK2"], Point<int16_t>(275, 395));
		buttons[Buttons::BT_CANCEL] = std::make_unique<MapleButton>(Basic["BtCancel2"], Point<int16_t>(340, 395));

		text = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, rules_text(), RULES_WIDTH);
		steps = std::max<int16_t>(0, (text.height() - RULES_VIEW + RULES_STEP - 1) / RULES_STEP);
		per_row = 1 + steps / RULES_SLIDER_ROWS;
		rows = (steps + per_row - 1) / per_row;

		int16_t top = RULES_TEXT_AT.y();
		slider = Slider(Slider::Type::CLASSIC, Range<int16_t>(top, top + RULES_SLIDER_HEIGHT), RULES_SLIDER_X, 1, rows + 1,
						[&](bool up) { scroll_to(step + (up ? -per_row : per_row)); });
	}

	void UINameChangeRules::draw(float inter) const
	{
		draw_from_corner(background, position);

		int16_t top = position.y() + RULES_TEXT_AT.y();
		text.draw(DrawArgument(position + RULES_TEXT_AT - Point<int16_t>(0, step * RULES_STEP)), Range<int16_t>(top, top + RULES_VIEW));
		slider.draw(position);

		UIElement::draw(inter);
	}

	Cursor::State UINameChangeRules::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		if (Cursor::State state = slider.send_cursor(cursorpos - position, clicked))
			return state;

		return UIElement::send_cursor(clicked, cursorpos);
	}

	void UINameChangeRules::send_scroll(double yoffset)
	{
		// The wheel goes a step at a time, finer than the slider's rows.
		if (yoffset != 0)
			scroll_to(step + (yoffset < 0 ? 1 : -1));
	}

	void UINameChangeRules::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (!pressed)
			return;

		if (escape)
			answer(false);
		else if (keycode == KeyAction::Id::RETURN)
			answer(true);
	}

	UIElement::Type UINameChangeRules::get_type() const
	{
		return TYPE;
	}

	Button::State UINameChangeRules::button_pressed(uint16_t buttonid)
	{
		answer(buttonid == Buttons::BT_OK);

		return Button::State::NORMAL;
	}

	void UINameChangeRules::scroll_to(int16_t to)
	{
		step = std::clamp<int16_t>(to, 0, steps);
		slider.setrows(step == steps ? rows : step / per_row, 1, rows + 1);
	}

	void UINameChangeRules::answer(bool ok)
	{
		deactivate();

		if (ok)
			UI::get().emplace<UINameChangeEntry>(sn);
	}

	UINameChangeEntry::UINameChangeEntry(int32_t sn) : UIElement(Point<int16_t>(0, 0), Point<int16_t>(0, 0)), sn(sn)
	{
		background = NxFiles::ui_classic["CashShop.img"]["CSChangeName"]["Base"]["backgrnd"];
		dimension = background.get_dimensions();
		position = centered(dimension);

		nl::node Basic = NxFiles::ui_classic["Basic.img"];

		buttons[Buttons::BT_CHECK] = std::make_unique<MapleButton>(NxFiles::ui_classic["CashShop.img"]["CSChangeName"]["BtCheck"], Point<int16_t>(195, 26));
		buttons[Buttons::BT_OK] = std::make_unique<MapleButton>(Basic["BtOK2"], Point<int16_t>(161, 93));
		buttons[Buttons::BT_CANCEL] = std::make_unique<MapleButton>(Basic["BtCancel2"], Point<int16_t>(210, 93));

		name = Textfield(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, NAME_BOX, name_change::NAME_LIMIT);
		name.set_enter_callback([&](std::string) { confirm(); });
		name.set_key_callback(KeyAction::Id::ESCAPE, [&]() { close(); });
		name.set_state(Textfield::State::FOCUSED);
	}

	void UINameChangeEntry::draw(float inter) const
	{
		draw_from_corner(background, position);
		name.draw(position);

		UIElement::draw(inter);
	}

	void UINameChangeEntry::update()
	{
		UIElement::update();

		name.update(position);
	}

	Cursor::State UINameChangeEntry::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		Cursor::State state = name.send_cursor(cursorpos, clicked);

		if (state != Cursor::State::IDLE)
			return state;

		return UIElement::send_cursor(clicked, cursorpos);
	}

	void UINameChangeEntry::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (!pressed)
			return;

		if (escape)
			close();
		else if (keycode == KeyAction::Id::RETURN)
			confirm();
	}

	UIElement::Type UINameChangeEntry::get_type() const
	{
		return TYPE;
	}

	Button::State UINameChangeEntry::button_pressed(uint16_t buttonid)
	{
		switch (buttonid)
		{
		case Buttons::BT_CHECK:
			check();
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

	void UINameChangeEntry::checked(const std::string& text, int8_t taken)
	{
		const char* refusal = name_change::check_answer(taken);

		// A name the server last refused can't be bought on an older yes.
		checked_name = refusal ? std::string() : text;

		notice(refusal ? refusal : name_change::CAN_USE);
	}

	void UINameChangeEntry::check()
	{
		std::string text = name.get_text();
		bool allowed = character_name_valid(text) && (Configuration::get().get_admin() || !CurseData::in_name(text));

		if (const char* refusal = name_change::check_refusal(text, Stage::get().get_player().get_name(), allowed))
			return notice(refusal);

		if (auto cashshop = UI::get().get_element<UICashShop>(); cashshop && cashshop->request())
			NameCharPacket(text).dispatch();
	}

	void UINameChangeEntry::confirm()
	{
		std::string text = name.get_text();
		std::string current = Stage::get().get_player().get_name();

		if (const char* refusal = name_change::buy_refusal(text, current, checked_name))
			return notice(refusal);

		if (auto cashshop = UI::get().get_element<UICashShop>(); cashshop && cashshop->request())
		{
			BuyNameChangePacket(sn, current, text).dispatch();
			close();
		}
	}

	void UINameChangeEntry::notice(const char* text)
	{
		// The notice takes the keys, Enter and Esc included.
		name.set_state(Textfield::State::NORMAL);
		UI::get().remove_textfield();

		UI::get().emplace<UIOk>(text, [](bool)
		{
			if (auto entry = UI::get().get_element<UINameChangeEntry>(); entry && entry->is_active())
				entry->name.set_state(Textfield::State::FOCUSED);
		});
	}

	void UINameChangeEntry::close()
	{
		name.set_state(Textfield::State::DISABLED);

		deactivate();
	}

	UICancelNameChange::UICancelNameChange(Kind kind, std::function<void(bool yes)> handler) : UIElement(Point<int16_t>(0, 0), Point<int16_t>(0, 0)), handler(handler), question(kind == ASK || kind == ASK_AGAIN)
	{
		nl::node requests = NxFiles::ui_classic["UIWindow.img"]["CancelRequests"];
		nl::node art = kind == ASK_AGAIN ? requests["AskContinueCancel"] : requests["CancelNameChange"];

		switch (kind)
		{
		case ASK:
		case ASK_AGAIN:
			background = art["backgrnd"];
			break;
		case CANCELLED:
			background = art["suc"];
			break;
		case FAILED:
			background = art["fail"];
			break;
		}

		dimension = background.get_dimensions();
		position = centered(dimension);

		if (question)
		{
			int16_t y = kind == ASK_AGAIN ? 120 : 101;

			buttons[Buttons::BT_YES] = std::make_unique<MapleButton>(art["BtYes"], Point<int16_t>(20, y));
			buttons[Buttons::BT_NO] = std::make_unique<MapleButton>(art["BtNo"], Point<int16_t>(148, y));
		}
		else
		{
			int16_t y = kind == FAILED ? 103 : 87;

			buttons[Buttons::BT_OK] = std::make_unique<MapleButton>(NxFiles::ui_classic["Basic.img"]["BtOK2"], Point<int16_t>(110, y));
		}

		Sound(Sound::Name::DLGNOTICE).play();
	}

	void UICancelNameChange::ask(std::function<void()> cancel)
	{
		UI::get().emplace<UICancelNameChange>(ASK, [cancel](bool yes)
		{
			if (yes)
				UI::get().emplace<UICancelNameChange>(ASK_AGAIN, [cancel](bool again)
				{
					if (again)
						cancel();
				});
		});
	}

	void UICancelNameChange::draw(float inter) const
	{
		draw_from_corner(background, position);

		UIElement::draw(inter);
	}

	void UICancelNameChange::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (!pressed)
			return;

		if (escape)
			answer(false);
		else if (keycode == KeyAction::Id::RETURN)
			answer(true);
	}

	UIElement::Type UICancelNameChange::get_type() const
	{
		return TYPE;
	}

	Button::State UICancelNameChange::button_pressed(uint16_t buttonid)
	{
		answer(buttonid != Buttons::BT_NO);

		return Button::State::NORMAL;
	}

	void UICancelNameChange::answer(bool yes)
	{
		deactivate();

		if (handler)
			handler(question && yes);
	}
}
