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

		static constexpr Type TYPE = UIElement::Type::NPCTALK;
		static constexpr bool FOCUSED = true;
		static constexpr bool TOGGLED = false;

		UINpcTalk();

		void draw(float inter) const override;

		void update() override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

		// `headers` puts an image (e.g. a quest list category) above menu choices.
		void change_text(int32_t npcid, int8_t msgtype, int16_t style, int8_t speaker, const std::string &text,
						 const std::map<int32_t, Texture> &headers = {});

		// Answer this dialogue in the client instead of sending NPC_TALK_MORE.
		// The responder gets 1/0/-1 (next or yes / previous or no / close) and,
		// for menus, the chosen #L id. change_text clears it.
		void set_local_responder(std::function<void(int8_t, int32_t)> responder);

	protected:
		Button::State button_pressed(uint16_t buttonid) override;

	private:
		TalkType get_by_value(int8_t value);

		// The text area's rows, relative to the window.
		Range<int16_t> visible_rows() const;

		// Send a response to the server, or to the local responder if set.
		void respond(int8_t response, int32_t selection = -1);


		std::string format_text(const std::string &tx, const int32_t &npcid);

		static constexpr int16_t MAX_HEIGHT = 248;

		enum Buttons
		{
			ALLLEVEL,
			CLOSE,
			MYLEVEL,
			NEXT,
			NO,
			OK,
			PREV,
			QAFTER,
			QCNO,
			QCYES,
			QGIVEUP,
			QNO,
			QSTART,
			QYES,
			YES
		};

		Texture top;
		Texture fill;
		Texture bottom;
		Texture nametag;
		Texture speaker;

		Text text;
		Text name;

		int16_t height;
		int16_t offset;
		int16_t unitrows;
		int16_t rowmax;
		int16_t min_height;

		bool show_slider;
		bool draw_text;
		Slider slider;
		TalkType type;
		int8_t message_type;
		struct Choice
		{
			int32_t id;
			Text label;
			Texture header;
			int16_t y = 0;
		};

		std::vector<Choice> choices;
		std::function<void(int8_t, int32_t)> local_responder;
		int16_t question_height;
		std::string formatted_text;
		size_t formatted_text_pos;
		uint16_t timestep;

		std::function<void(bool)> onmoved;
	};
}
