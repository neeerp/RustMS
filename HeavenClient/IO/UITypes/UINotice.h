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

#include <string>

#include <cstdint>

#include "../UIDragElement.h"

#include "../Components/Textfield.h"

#include "../../Graphics/Geometry.h"

namespace ms
{
	class UINotice : public UIDragElement<PosNOTICE>
	{
	public:
		static constexpr Type TYPE = UIElement::Type::NOTICE;
		static constexpr bool FOCUSED = true;
		static constexpr bool TOGGLED = false;

	protected:
		enum NoticeType : uint8_t
		{
			YESNO,
			ENTERNUMBER,
			OK
		};

		UINotice(std::string message, NoticeType type, Text::Alignment alignment);

		UINotice(std::string message, NoticeType type);

		// Draws the box and the message. The message sits in the text frame, above the field for number entry.
		void draw() const;

		// The top edge of the button frames at the bottom of the box.
		int16_t buttons_y() const;

		// The top edge of the number field, below the message.
		int16_t field_y() const;

	private:
		Texture top;
		Texture center;
		Texture bottom;
		Text question;
		int16_t rows;
		NoticeType type;
		Text::Alignment alignment;
	};

	class UIYesNo : public UINotice
	{
	public:
		UIYesNo(std::string message, std::function<void(bool yes)> yesnohandler, Text::Alignment alignment);

		UIYesNo(std::string message, std::function<void(bool yes)> yesnohandler);

		void draw(float alpha) const override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		enum Buttons : int16_t
		{
			YES, NO
		};

		std::function<void(bool yes)> yesnohandler;
	};

	class UIEnterNumber : public UINotice
	{
	public:
		// Its own type, so that a notice about the number opens over it instead of replacing it.
		static constexpr Type TYPE = UIElement::Type::ENTERNUMBER;

		UIEnterNumber(std::string message, std::function<void(int32_t number)> numhandler, int32_t max,
					  int32_t quantity);

		void draw(float alpha) const override;

		void update() override;

		Cursor::State send_cursor(bool pressed, Point<int16_t> cursorpos) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		void handlestring(std::string numstr);

		// Close and give the keys back to the game.
		void close();

		enum Buttons : int16_t
		{
			OK, CANCEL
		};

		std::function<void(int32_t number)> numhandler;
		ColorBox fieldborder;
		ColorBox fieldfill;
		Textfield numfield;
		int32_t max;
	};

	// Asks for a line of text, such as a character name.
	class UIEnterText : public UINotice
	{
	public:
		UIEnterText(std::string message, std::function<void(const std::string &text)> texthandler, size_t limit);

		void draw(float alpha) const override;

		void update() override;

		Cursor::State send_cursor(bool pressed, Point<int16_t> cursorpos) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		void submit();

		void close();

		enum Buttons : int16_t
		{
			OK, CANCEL
		};

		std::function<void(const std::string &text)> texthandler;
		Textfield textfield;
	};

	class UIOk : public UINotice
	{
	public:
		UIOk(std::string message, std::function<void(bool ok)> okhandler);

		void draw(float alpha) const override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		enum Buttons : int16_t
		{
			OK
		};

		std::function<void(bool ok)> okhandler;
	};
}