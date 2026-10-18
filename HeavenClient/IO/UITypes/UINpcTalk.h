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

#include <cstddef>

#include <cstdint>

#include "../UIElement.h"

#include "../Graphics/Text.h"
#include "../Components/AnswerFocus.h"
#include "../Components/Slider.h"

#include <functional>
#include <map>

namespace ms
{
	class UINpcTalk : public UIElement
	{
	public:
		enum TalkType : int8_t
		{
			NONE = -1,
			SENDOK,
			SENDYESNO,

			// TODO: Unconfirmed
					SENDNEXT,
			SENDNEXTPREV,
			SENDACCEPTDECLINE,
			SENDGETTEXT,
			SENDGETNUMBER,
			SENDSIMPLE,
			LENGTH
		};

		// Where a menu shows its choices: in the text, as the server's
		// menus do, or listed under it 32 px apart, as the client's quest
		// list and quest questions do.
		enum class Menu
		{
			INLINE,
			LIST
		};

		static constexpr Type TYPE = UIElement::Type::NPCTALK;
		static constexpr bool FOCUSED = true;
		static constexpr bool TOGGLED = false;

		UINpcTalk();

		void draw(float inter) const override;

		void update() override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;

		void send_scroll(double yoffset) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

		// `headers` puts an image (e.g. a quest list category) above listed
		// menu choices.
		void change_text(int32_t npcid, int8_t msgtype, int16_t style, int8_t speaker, const std::string &text,
						 const std::map<int32_t, Texture> &headers = {}, Menu menu = Menu::INLINE);

		// Answer this dialogue in the client instead of sending NPC_TALK_MORE.
		// The responder gets 1/0/-1 (next or yes / previous or no / close) and,
		// for menus, the chosen #L id. change_text clears it.
		void set_local_responder(std::function<void(int8_t, int32_t)> responder);

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		TalkType get_by_value(int8_t value);

		// Where the text starts, relative to the window, after scrolling.
		Point<int16_t> text_origin() const;

		// The text's left edge, relative to the window.
		int16_t text_x() const;

		// The text area's rows, relative to the window.
		Range<int16_t> visible_rows() const;

		// How much of the text has typed out.
		size_t revealed() const;

		// The choice under a point relative to the window, or -1.
		int16_t choice_at(Point<int16_t> cursor) const;

		// Selects a choice and scrolls it into view.
		void select(size_t choice);

		// Answers a menu with the selected choice.
		void answer_choice();

		// Send a response to the server, or to the local responder if set.
		void respond(int8_t response, int32_t selection = -1);

		// An image in the text (#f, #i, #s, #v): its box in the layout, and
		// where the texture's origin goes from the box's top-left corner.
		struct InlineImage
		{
			Texture texture;
			Point<int16_t> size;
			Point<int16_t> anchor;
			size_t box = 0;
		};

		// Expands the markup, leaving a Text::BOX for each image in `images`.
		std::string format_text(const std::string &tx, const int32_t &npcid, std::vector<InlineImage> &images);

		static constexpr int16_t MIN_HEIGHT = 120;
		static constexpr int16_t MAX_HEIGHT = 240;

		enum Buttons
		{
			CLOSE,
			NEXT,
			NO,
			OK,
			PREV,
			QNO,
			QYES,
			YES
		};

		// The page's two answers (YES/NO or ACCEPT/DECLINE), left first, if
		// it has them.
		bool answers(Buttons &left, Buttons &right) const;

		// Outlines the answer the keyboard has focused.
		void focus_answer(answer_focus::Focus focus);

		Texture top;
		Texture fill;
		Texture bottom;
		Texture nametag;
		Texture speaker;
		Texture dot_selected;
		Texture dot;

		Text text;
		Text name;

		// The height of the middle, between top and bottom.
		int16_t height;
		// The text's top before scrolling, relative to the window.
		int16_t text_y;
		// Lines scrolled, and the most there are.
		int16_t offset;
		int16_t rowmax;

		bool show_slider;
		bool draw_text;
		// From the speaker byte: who speaks, on which side, and whether the
		// NPC is mirrored.
		bool player_speaks = false;
		bool right_side = false;
		bool flip_speaker = false;
		Slider slider;
		TalkType type;
		Menu menu_kind;
		int8_t message_type;

		struct Choice
		{
			int32_t id;
			// Where the choice starts, on its baseline: the dot goes there and
			// the label follows it. And where the label ends. Both relative
			// to the text.
			Point<int16_t> start;
			Point<int16_t> end;
			// Listed choices draw their own label, under an optional header.
			Text label;
			Texture header;
			int16_t header_y = 0;
		};

		std::vector<Choice> choices;
		// Text that shows at once while the rest types out, in order: inline
		// choices, as in footage, and what v83 puts between #w marks.
		std::vector<std::pair<size_t, size_t>> instant_text;
		size_t selected;
		answer_focus::Focus focus;
		std::vector<InlineImage> images;
		std::function<void(int8_t, int32_t)> local_responder;
		std::string formatted_text;
		size_t formatted_text_pos;
		uint16_t timestep;

		std::function<void(bool)> onmoved;
	};
}
