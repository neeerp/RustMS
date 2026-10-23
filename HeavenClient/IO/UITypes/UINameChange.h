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

#include "../Components/Slider.h"
#include "../Components/Textfield.h"

#include <functional>

namespace ms
{
	// Name Change's BUY (v83 47031e): the birthday code on CSNotice/1,
	// then the NX Prepaid must cover the price before the rules are asked
	// for (op 0x10).
	class UINameChangeBirthday : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::NOTICE;
		static constexpr bool FOCUSED = true;
		static constexpr bool TOGGLED = false;

		UINameChangeBirthday(int32_t sn, int32_t price, int32_t prepaid);

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
			BT_CANCEL
		};

		Texture background;
		Textfield birthday;
		int32_t sn;
		int32_t price;
		int32_t prepaid;
	};

	// The rules sheet (v83 dialog 007ee39b on CSChangeName/Base/
	// backgrndnotice): String NameChange.img's 31 paragraphs, scrolled 7
	// pixels a step. OK goes on to the name.
	class UINameChangeRules : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::NOTICE;
		static constexpr bool FOCUSED = true;
		static constexpr bool TOGGLED = false;

		UINameChangeRules(int32_t sn);

		void draw(float inter) const override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;

		void send_scroll(double yoffset) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		void scroll_to(int16_t to);

		void answer(bool ok);

		enum Buttons : uint16_t
		{
			BT_OK,
			BT_CANCEL
		};

		Texture background;
		Text text;
		Slider slider;
		int16_t step;
		int16_t steps;
		// The slider's rows, of per_row steps each.
		int16_t rows;
		int16_t per_row;
		int32_t sn;
	};

	// The new name (v83 dialog 007eda8c on CSChangeName/Base/backgrnd):
	// CHECK asks the server (op 0x15), and OK buys the name it last said
	// could be used. Its own type, so that its notices open over it.
	class UINameChangeEntry : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::NAMECHANGE;
		static constexpr bool FOCUSED = true;
		static constexpr bool TOGGLED = false;

		UINameChangeEntry(int32_t sn);

		void draw(float inter) const override;

		void update() override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

		// The server's answer to CHECK (0x148).
		void checked(const std::string& name, int8_t taken);

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		void check();

		void confirm();

		// A notice over the dialog, which has the name again once it
		// closes.
		void notice(const char* text);

		void close();

		enum Buttons : uint16_t
		{
			BT_CHECK,
			BT_OK,
			BT_CANCEL
		};

		Texture background;
		Textfield name;
		std::string checked_name;
		int32_t sn;
	};

	// The image dialogs of cancelling with the receipt (v83 998874 and
	// 998c81, UIWindow.img/CancelRequests): the two questions, and the
	// answer.
	class UICancelNameChange : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::NOTICE;
		static constexpr bool FOCUSED = true;
		static constexpr bool TOGGLED = false;

		enum Kind
		{
			// "Are you sure you want to cancel the name change request?"
			ASK,
			// "There will not be any refund", asked second.
			ASK_AGAIN,
			CANCELLED,
			FAILED
		};

		UICancelNameChange(Kind kind, std::function<void(bool yes)> handler);

		// Ask both questions, then call `cancel`.
		static void ask(std::function<void()> cancel);

		void draw(float inter) const override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		void answer(bool yes);

		enum Buttons : uint16_t
		{
			BT_YES,
			BT_NO,
			BT_OK
		};

		Texture background;
		std::function<void(bool yes)> handler;
		bool question;
	};
}
