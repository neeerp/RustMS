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
#include "UINpcTalk.h"

#include "../UI.h"

#include "../Components/MapleButton.h"
#include "../Gameplay/Stage.h"

#include "../Net/Packets/NpcInteractionPackets.h"

#include <nlnx/nx.hpp>

namespace ms
{
	UINpcTalk::UINpcTalk() : offset(0), unitrows(0), rowmax(0), show_slider(false), draw_text(false),
							 formatted_text(""), formatted_text_pos(0), timestep(0)
	{
		nl::node UIWindow2 = nl::nx::ui["UIWindow2.img"];
		nl::node UtilDlgEx = UIWindow2["UtilDlgEx"];

		top = UtilDlgEx["t"];
		fill = UtilDlgEx["c"];
		bottom = UtilDlgEx["s"];
		nametag = UtilDlgEx["bar"];

		min_height = 8 * fill.height() + 14;

		buttons[Buttons::ALLLEVEL] = std::make_unique<MapleButton>(UtilDlgEx["BtAllLevel"]);
		buttons[Buttons::CLOSE] = std::make_unique<MapleButton>(UtilDlgEx["BtClose"]);
		buttons[Buttons::MYLEVEL] = std::make_unique<MapleButton>(UtilDlgEx["BtMyLevel"]);
		buttons[Buttons::NEXT] = std::make_unique<MapleButton>(UtilDlgEx["BtNext"]);

		// TODO: Replace when _inlink is fixed
		//buttons[Buttons::NO] = std::make_unique<MapleButton>(UtilDlgEx["BtNo"]);

		nl::node Quest = UIWindow2["Quest"];

		buttons[Buttons::NO] = std::make_unique<MapleButton>(Quest["BtNo"]);
		buttons[Buttons::OK] = std::make_unique<MapleButton>(UtilDlgEx["BtOK"]);
		buttons[Buttons::PREV] = std::make_unique<MapleButton>(UtilDlgEx["BtPrev"]);
		buttons[Buttons::QAFTER] = std::make_unique<MapleButton>(UtilDlgEx["BtQAfter"]);
		buttons[Buttons::QCNO] = std::make_unique<MapleButton>(UtilDlgEx["BtQCNo"]);
		buttons[Buttons::QCYES] = std::make_unique<MapleButton>(UtilDlgEx["BtQCYes"]);
		buttons[Buttons::QGIVEUP] = std::make_unique<MapleButton>(UtilDlgEx["BtQGiveup"]);
		buttons[Buttons::QNO] = std::make_unique<MapleButton>(UtilDlgEx["BtQNo"]);
		buttons[Buttons::QSTART] = std::make_unique<MapleButton>(UtilDlgEx["BtQStart"]);
		buttons[Buttons::QYES] = std::make_unique<MapleButton>(UtilDlgEx["BtQYes"]);
		buttons[Buttons::YES] = std::make_unique<MapleButton>(UtilDlgEx["BtYes"]);

		name = Text(Text::Font::A11M, Text::Alignment::CENTER, Color::Name::WHITE);

		onmoved = [&](bool upwards)
		{
			int16_t shift = upwards ? -unitrows : unitrows;
			bool above = offset + shift >= 0;
			bool below = offset + shift <= rowmax - unitrows;

			if (above && below)
				offset += shift;
		};

		UI::get().remove_textfield();
	}

	void UINpcTalk::draw(float inter) const
	{
		Point<int16_t> drawpos = position;
		top.draw(drawpos);
		drawpos.shift_y(top.height());
		fill.draw(DrawArgument(drawpos, Point<int16_t>(0, height)));
		drawpos.shift_y(height);
		bottom.draw(drawpos);
		drawpos.shift_y(bottom.height());

		UIElement::draw(inter);

		int16_t speaker_y = (top.height() + height + bottom.height()) / 2;
		Point<int16_t> speaker_pos = position + Point<int16_t>(22, 11 + speaker_y);
		Point<int16_t> center_pos = speaker_pos + Point<int16_t>(nametag.width() / 2, 0);

		speaker.draw(DrawArgument(center_pos, true));
		nametag.draw(speaker_pos);
		name.draw(center_pos + Point<int16_t>(0, -4));

		if (show_slider)
		{
			int16_t text_min_height = position.y() + top.height() - 1;
			text.draw(position + Point<int16_t>(166, 32 - offset * 32),
					  Range<int16_t>(text_min_height, text_min_height + height - 18));
			slider.draw(position);
		} else
		{
			text.draw(position + Point<int16_t>(166, 32));
		}
		for (const auto &choice : choices)
		{
			Point<int16_t> at = position + Point<int16_t>(166, choice.y - offset * 32);

			if (choice.header.is_valid())
				choice.header.draw(at - Point<int16_t>(0, choice.header.height() + 2));

			choice.label.draw(at);
		}
	}

	void UINpcTalk::update()
	{
		UIElement::update();

		if (draw_text)
		{
			if (timestep > 4)
			{
				if (formatted_text_pos < formatted_text.size())
				{
					std::string t = text.get_text();
					char c = formatted_text[formatted_text_pos];

					text.change_text(t + c);

					formatted_text_pos++;
					timestep = 0;
				} else
				{
					draw_text = false;
				}
			} else
			{
				timestep++;
			}
		}
	}

	Button::State UINpcTalk::button_pressed(uint16_t buttonid)
	{
		deactivate();

		switch (type)
		{
			case TalkType::SENDNEXT:
			case TalkType::SENDOK:
			case TalkType::SENDNEXTPREV:
				// Type = 0
				switch (buttonid)
				{
					case Buttons::CLOSE:
						respond(-1);
						break;
					case Buttons::NEXT:
					case Buttons::OK:
						respond(1);
						break;
					case Buttons::PREV:
						respond(0);
						break;
				}
				break;
			case TalkType::SENDYESNO:
				// Type = 1
				switch (buttonid)
				{
					case Buttons::CLOSE:
						respond(-1);
						break;
					case Buttons::NO:
						respond(0);
						break;
					case Buttons::YES:
						respond(1);
						break;
				}
				break;
			case TalkType::SENDACCEPTDECLINE:
				// Type = 1
				switch (buttonid)
				{
					case Buttons::CLOSE:
						respond(-1);
						break;
					case Buttons::QNO:
						respond(0);
						break;
					case Buttons::QYES:
						respond(1);
						break;
				}
				break;
			case TalkType::SENDGETTEXT:
				// TODO: What is this?
				break;
			case TalkType::SENDGETNUMBER:
				// Type = 3
				switch (buttonid)
				{
					case Buttons::CLOSE:
						respond(0);
						break;
					case Buttons::OK:
						respond(1);
						break;
				}
				break;
			case TalkType::SENDSIMPLE:
				// Type = 4
				if (buttonid == Buttons::CLOSE)
					local_responder ? respond(-1) : respond(0);
				break;
			default:
				break;
		}

		return Button::State::NORMAL;
	}

	void UINpcTalk::respond(int8_t response, int32_t selection)
	{
		if (local_responder)
		{
			// The responder may replace this dialogue, so call a copy.
			auto responder = local_responder;
			responder(response, selection);
			return;
		}

		if (selection >= 0)
			NpcTalkMorePacket(selection).dispatch();
		else
			NpcTalkMorePacket(message_type, response).dispatch();
	}

	void UINpcTalk::set_local_responder(std::function<void(int8_t, int32_t)> responder)
	{
		local_responder = std::move(responder);
	}

	Cursor::State UINpcTalk::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		Point<int16_t> cursor_relative = cursorpos - position;
		if (message_type == 4)
			for (const auto &choice : choices)
			{
				int16_t y = choice.y - offset * 32;
				if (Rectangle<int16_t>(166, 486, y, y + 24).contains(cursor_relative))
				{
					if (clicked)
					{
						deactivate();
						respond(1, choice.id);
						return Cursor::State::IDLE;
					}
					return Cursor::State::CANCLICK;
				}
			}

		if (show_slider && slider.isenabled())
			if (Cursor::State sstate = slider.send_cursor(cursor_relative, clicked))
				return sstate;
		if (clicked && draw_text && Rectangle<int16_t>(166, 486, 24, height + 24).contains(cursor_relative))
		{
			draw_text = false;
			text.change_text(formatted_text);
			return Cursor::State::IDLE;
		}

		Cursor::State estate = UIElement::send_cursor(clicked, cursorpos);

		if (estate == Cursor::State::CLICKING && clicked && draw_text)
		{
			draw_text = false;
			text.change_text(formatted_text);
		}

		return estate;
	}

	void UINpcTalk::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (pressed && escape)
		{
			deactivate();

			respond(-1);
		}
	}

	UIElement::Type UINpcTalk::get_type() const
	{
		return TYPE;
	}

	UINpcTalk::TalkType UINpcTalk::get_by_value(int8_t value)
	{
		if (value > TalkType::NONE && value < TalkType::LENGTH)
			return static_cast<TalkType>(value);

		return TalkType::NONE;
	}

	// TODO: Move this to GraphicsGL?
	std::string UINpcTalk::format_text(const std::string &tx, const int32_t &npcid)
	{
		std::string formatted_text = tx;
		// The text renderer understands escaped line breaks, not literal LF/CR.
		// Quest prompts may contain both asset escapes and real newlines.
		formatted_text.clear();
		for (size_t i = 0; i < tx.size(); ++i)
		{
			if (tx[i] == '\r' || tx[i] == '\n')
			{
				if (tx[i] == '\r' && i + 1 < tx.size() && tx[i + 1] == '\n') ++i;
				formatted_text += "\\n";
			}
			else formatted_text += tx[i];
		}
		size_t begin = formatted_text.find("#p");

		if (begin != std::string::npos)
		{
			size_t end = formatted_text.find("#", begin + 1);

			if (end != std::string::npos)
			{
				std::string namestr = nl::nx::string["Npc.img"][std::to_string(npcid)]["name"];
				formatted_text.replace(begin, end - begin, namestr);
			}
		}

		begin = formatted_text.find("#h");

		if (begin != std::string::npos)
		{
			size_t end = formatted_text.find("#", begin + 1);

			if (end != std::string::npos)
			{
				std::string charstr = Stage::get().get_player().get_name();
				formatted_text.replace(begin, end - begin + 1, charstr);
			}
		}

		begin = formatted_text.find("#t");

		if (begin != std::string::npos)
		{
			size_t end = formatted_text.find("#", begin + 1);

			if (end != std::string::npos)
			{
				size_t b = begin + 2;
				int32_t itemid = std::stoi(formatted_text.substr(b, end - b));
				std::string itemname = nl::nx::string["Consume.img"][itemid]["name"];

				formatted_text.replace(begin, end - begin, itemname);
			}
		}

		return formatted_text;
	}

	void UINpcTalk::change_text(int32_t npcid, int8_t msgtype, int16_t style, int8_t speakerbyte, const std::string &tx,
								const std::map<int32_t, Texture> &headers)
	{
		local_responder = nullptr;
		message_type = msgtype;
		type = get_by_value(msgtype);
		if (msgtype == 4)
			type = TalkType::SENDSIMPLE;
		const bool has_previous = (style & 0xff) != 0;
		const bool has_next = (style & 0xff00) != 0;
		if (msgtype == 0)
			type = has_previous ? TalkType::SENDNEXTPREV : has_next ? TalkType::SENDNEXT : TalkType::SENDOK;

		timestep = 0;
		draw_text = true;
		formatted_text_pos = 0;
		formatted_text = format_text(tx, npcid);
		choices.clear();
		if (msgtype == 4)
		{
			size_t first_link = formatted_text.find("#L");
			for (size_t start = first_link; start != std::string::npos;)
			{
				size_t id_end = formatted_text.find('#', start + 2);
				size_t label_end = formatted_text.find("#l", id_end);
				if (id_end == std::string::npos || label_end == std::string::npos)
					break;
				int32_t id = std::stoi(formatted_text.substr(start + 2, id_end - start - 2));
				std::string label = formatted_text.substr(id_end + 1, label_end - id_end - 1);
				auto header = headers.find(id);
				choices.push_back({ id, Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLUE, label, 320),
					header == headers.end() ? Texture() : header->second });
				start = formatted_text.find("#L", label_end + 2);
			}
			if (first_link != std::string::npos)
				formatted_text.erase(first_link);
		}

		text = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::DARKGREY, formatted_text, 320);

		int16_t text_height = text.height();
		question_height = text_height;

		int16_t choice_y = 40 + question_height;

		for (auto &choice : choices)
		{
			if (choice.header.is_valid())
				choice_y += choice.header.height() + 6;

			choice.y = choice_y;
			choice_y += 28;
		}

		text_height = choice_y - 8;

		text.change_text("");

		if (speakerbyte == 0)
		{
			std::string strid = std::to_string(npcid);
			strid.insert(0, 7 - strid.size(), '0');
			strid.append(".img");

			speaker = nl::nx::npc[strid]["stand"]["0"];

			std::string namestr = nl::nx::string["Npc.img"][std::to_string(npcid)]["name"];
			name.change_text(namestr);
		} else
		{
			speaker = Texture();
			name.change_text("");
		}

		height = min_height;
		show_slider = false;

		if (text_height > height)
		{
			if (text_height > MAX_HEIGHT)
			{
				height = MAX_HEIGHT;
				show_slider = true;
				rowmax = (text_height - MAX_HEIGHT + 31) / 32 + 1;
				unitrows = 1;

				int16_t slider_y = top.height() - 7;
				slider = Slider(Slider::Type::DEFAULT_SILVER, Range<int16_t>(slider_y, slider_y + height - 20),
								top.width() - 26, unitrows, rowmax, onmoved);
			} else
			{
				height = text_height;
			}
		}

		for (auto &button : buttons)
		{
			button.second->set_active(false);
			button.second->set_state(Button::State::NORMAL);
		}

		int16_t y_cord = height + 48;

		buttons[Buttons::CLOSE]->set_position(Point<int16_t>(9, y_cord));
		buttons[Buttons::CLOSE]->set_active(true);

		switch (type)
		{
			case TalkType::SENDOK:
				buttons[Buttons::OK]->set_position(Point<int16_t>(471, y_cord));
				buttons[Buttons::OK]->set_active(true);
				break;
			case TalkType::SENDYESNO:
			{
				Point<int16_t> yes_position = Point<int16_t>(389, y_cord);

				buttons[Buttons::YES]->set_position(yes_position);
				buttons[Buttons::YES]->set_active(true);

				buttons[Buttons::NO]->set_position(yes_position + Point<int16_t>(65, 0));
				buttons[Buttons::NO]->set_active(true);
				break;
			}
			case TalkType::SENDNEXT:
			case TalkType::SENDNEXTPREV:
				if (has_previous)
				{
					buttons[Buttons::PREV]->set_position(Point<int16_t>(389, y_cord));
					buttons[Buttons::PREV]->set_active(true);
				}
				buttons[has_next ? Buttons::NEXT : Buttons::OK]->set_position(Point<int16_t>(471, y_cord));
				buttons[has_next ? Buttons::NEXT : Buttons::OK]->set_active(true);
				break;
			case TalkType::SENDACCEPTDECLINE:
			case TalkType::SENDGETTEXT:
			case TalkType::SENDGETNUMBER:
			case TalkType::SENDSIMPLE:
			default:
				break;
		}

		position = Point<int16_t>(400 - top.width() / 2, 240 - height / 2);
		dimension = Point<int16_t>(top.width(), height + 120);
	}
}
