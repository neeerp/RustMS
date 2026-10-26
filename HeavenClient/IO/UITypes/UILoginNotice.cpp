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
#include "UILoginNotice.h"

#include "../UI.h"

#include "../Components/MapleButton.h"

#include "../../Util/NxFiles.h"

#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// The v83 notice is a 362x219 scroll centred on the screen, with the
		// message's text, which has the parchment baked in, right of the
		// mushroom stamp. These match v0.50 and v0.55 footage at 800x600.
		const Point<int16_t> POSITION = Point<int16_t>(219, 170);
		const Point<int16_t> TEXT_POS = Point<int16_t>(119, 21);
		const Point<int16_t> SINGLE_POS = Point<int16_t>(162, 150);
		const Point<int16_t> PAIR_POS[2] = { Point<int16_t>(106, 150), Point<int16_t>(195, 150) };
	}

	UILoginNotice::UILoginNotice(uint16_t message, std::function<void()> okhandler, std::function<void()> cancelhandler)
			: okhandler(okhandler), cancelhandler(cancelhandler)
	{
		multiple = false;

		nl::node Notice = NxFiles::ui_classic["Login.img"]["Notice"];
		nl::node backgrnd;

		// Errors get the "no" stamp, as in v0.55 footage of a login error.
		switch (message)
		{
			case Message::NAME_OK:
			case Message::DELETE_CONFIRMATION:
			case Message::CONFIRM_EXIT:
			case Message::CASH_ITEMS_CONFIRM_DELETION:
			case Message::FAMILY_CONFIRM_DELETION:
			case Message::FAMILY_AND_CASH_ITEMS_CONFIRM_DELETION:
			// v83 opens this one with the plain background (FUN_0060f024(0x24, 0)).
			case Message::POPULATION_TOO_HIGH:
				backgrnd = Notice["backgrnd"]["0"];
				break;
			default:
				backgrnd = Notice["backgrnd"]["1"];
				break;
		}

		sprites.emplace_back(backgrnd);
		sprites.emplace_back(Notice["text"][message], TEXT_POS);

		if (message == Message::DELETE_CONFIRMATION)
		{
			multiple = true;

			buttons[Buttons::YES] = std::make_unique<MapleButton>(Notice["BtYes"], PAIR_POS[0]);
			buttons[Buttons::NO] = std::make_unique<MapleButton>(Notice["BtNo"], PAIR_POS[1]);
		} else
		{
			buttons[Buttons::YES] = std::make_unique<MapleButton>(Notice["BtYes"], SINGLE_POS);
		}

		position = POSITION;
		dimension = Texture(backgrnd).get_dimensions();
	}

	UILoginNotice::UILoginNotice(uint16_t message, std::function<void()> okhandler) : UILoginNotice(message, okhandler,
																									[]()
																									{})
	{}

	UILoginNotice::UILoginNotice(uint16_t message) : UILoginNotice(message, []()
	{})
	{}

	void UILoginNotice::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (pressed)
		{
			if (escape)
			{
				if (!multiple)
					okhandler();
				else
					cancelhandler();

				deactivate();
			} else if (keycode == KeyAction::RETURN)
			{
				okhandler();
				deactivate();
			}
		}
	}

	UIElement::Type UILoginNotice::get_type() const
	{
		return TYPE;
	}

	Button::State UILoginNotice::button_pressed(uint16_t buttonid)
	{
		if (buttonid == Buttons::YES)
			okhandler();
		else if (buttonid == Buttons::NO)
			cancelhandler();

		deactivate();

		return Button::State::NORMAL;
	}

	UIQuitConfirm::UIQuitConfirm()
	{
		nl::node notice = NxFiles::ui_classic["Login.img"]["Notice"];
		nl::node backgrnd = notice["backgrnd"]["0"];

		sprites.emplace_back(backgrnd);
		sprites.emplace_back(notice["text"][UILoginNotice::Message::CONFIRM_EXIT], TEXT_POS);

		buttons[Buttons::BT_OK] = std::make_unique<MapleButton>(notice["BtYes"], PAIR_POS[0]);
		buttons[Buttons::BT_CANCEL] = std::make_unique<MapleButton>(notice["BtNo"], PAIR_POS[1]);

		position = POSITION;
		dimension = Texture(backgrnd).get_dimensions();
	}

	void UIQuitConfirm::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (pressed)
		{
			if (escape)
			{
				deactivate();
			} else if (keycode == KeyAction::RETURN)
			{
				UI::get().quit();
				deactivate();
			}
		}
	}

	UIElement::Type UIQuitConfirm::get_type() const
	{
		return TYPE;
	}

	Button::State UIQuitConfirm::button_pressed(uint16_t buttonid)
	{
		if (buttonid == BT_OK)
			UI::get().quit();

		deactivate();

		return Button::PRESSED;
	}
}