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
#include "UIChatBar.h"

#include "UIStatusBar.h"

#include "../Components/MapleButton.h"

#include "../../Configuration.h"
#include "../../Gameplay/Stage.h"
#include "../../Graphics/GraphicsGL.h"
#include "../../Net/Packets/MessagingPackets.h"
#include "../../Util/NxFiles.h"

namespace ms
{
	namespace
	{
		enum class WhisperParseResult
		{
			NotWhisper,
			InvalidWhisper,
			ValidWhisper
		};

		WhisperParseResult parse_whisper_command(
			const std::string& input,
			std::string& target,
			std::string& message
		)
		{
			static const std::string SHORT_COMMAND = "/w";
			static const std::string LONG_COMMAND = "/whisper";

			size_t command_length = 0;

			if (input == SHORT_COMMAND || input.rfind(SHORT_COMMAND + " ", 0) == 0)
				command_length = SHORT_COMMAND.length();
			else if (input == LONG_COMMAND || input.rfind(LONG_COMMAND + " ", 0) == 0)
				command_length = LONG_COMMAND.length();
			else
				return WhisperParseResult::NotWhisper;

			size_t target_start = input.find_first_not_of(' ', command_length);
			if (target_start == std::string::npos)
				return WhisperParseResult::InvalidWhisper;

			size_t target_end = input.find(' ', target_start);
			target = input.substr(target_start, target_end - target_start);
			if (target.empty() || target_end == std::string::npos)
				return WhisperParseResult::InvalidWhisper;

			size_t message_start = input.find_first_not_of(' ', target_end);
			if (message_start == std::string::npos)
				return WhisperParseResult::InvalidWhisper;

			message = input.substr(message_start);
			return message.empty() ? WhisperParseResult::InvalidWhisper : WhisperParseResult::ValidWhisper;
		}

		// Lines sit on a dark band, and so does the latest line in the strip
		// while the input field is closed.
		constexpr float BAND_ALPHA = 0.55f;
	}

	UIChatBar::UIChatBar()
	{
		chatopen = Setting<Chatopen>::get().load();
		chatfieldopen = false;
		lastpos = 0;
		rowpos = 0;
		rowmax = -1;
		arrow_state[Arrow::PREV] = 0;
		arrow_state[Arrow::NEXT] = 0;

		nl::node base = NxFiles::ui_classic["StatusBar.img"]["base"];
		nl::node basic = NxFiles::ui_classic["Basic.img"];
		nl::node scrollbar = basic["VScr4"];

		backgrnd = base["backgrnd"];
		backgrnd2 = base["backgrnd2"];
		chattarget = base["chatTarget"];
		chattarget_label = Text(Text::Font::A11M, Text::Alignment::LEFT, Color::Name::WHITE, "To All");
		chat_target = Target::ALL;

		for (size_t i = 0; i < 3; i++)
		{
			arrows[Arrow::PREV][i] = scrollbar["enabled"]["prev" + std::to_string(i)];
			arrows[Arrow::NEXT][i] = scrollbar["enabled"]["next" + std::to_string(i)];
		}

		arrows_disabled[Arrow::PREV] = scrollbar["disabled"]["prev"];
		arrows_disabled[Arrow::NEXT] = scrollbar["disabled"]["next"];
		scrollbase = scrollbar["enabled"]["base"];

		buttons[Buttons::BT_OPENCHAT] = std::make_unique<MapleButton>(basic["BtMax"], Point<int16_t>(536, 12));
		buttons[Buttons::BT_CLOSECHAT] = std::make_unique<MapleButton>(basic["BtMin"], Point<int16_t>(536, 12));
		buttons[Buttons::BT_OPENCHAT]->set_active(!chatopen);
		buttons[Buttons::BT_CLOSECHAT]->set_active(chatopen);

		chatfield = Textfield(Text::A12M, Text::LEFT, Color::Name::BLACK,
							  Rectangle<int16_t>(Point<int16_t>(87, 5), Point<int16_t>(530, 25)), 0);
		chatfield.set_state(Textfield::State::DISABLED);

		chatfield.set_enter_callback(
				[&](std::string msg)
				{
					if (msg.size() > 0)
					{
						size_t last = msg.find_last_not_of(' ');

						if (last != std::string::npos)
						{
							msg.erase(last + 1);
							std::string target;
							std::string message;
							WhisperParseResult whisper_result = parse_whisper_command(msg, target, message);

							if (whisper_result == WhisperParseResult::ValidWhisper)
							{
								WhisperPacket(target, message).dispatch();
								send_chatline("#g" + target + "<<" + message, LineType::WHITE);
							}
							else if (whisper_result == WhisperParseResult::InvalidWhisper)
							{
								send_chatline("Usage: /w <name> <message>", LineType::RED);
							}
							else if (chat_target == Target::PARTY)
							{
								send_party_chat(msg);
							}
							else if (chat_target == Target::BUDDY)
							{
								send_buddy_chat(msg);
							}
							else
							{
								GeneralChatPacket(msg, true).dispatch();
							}

							lastentered.push_back(msg);
							lastpos = lastentered.size();
						} else
						{
							toggle_chatfield();
						}

						chatfield.change_text("");
					} else
					{
						toggle_chatfield();
					}
				}
		);

		chatfield.set_key_callback(
				KeyAction::Id::UP,
				[&]()
				{
					if (lastpos > 0)
					{
						lastpos--;
						chatfield.change_text(lastentered[lastpos]);
					}
				}
		);

		chatfield.set_key_callback(
				KeyAction::Id::DOWN,
				[&]()
				{
					if (lastentered.size() > 0 && lastpos < lastentered.size() - 1)
					{
						lastpos++;
						chatfield.change_text(lastentered[lastpos]);
					}
				}
		);

		chatfield.set_key_callback(
				KeyAction::Id::ESCAPE,
				[&]()
				{
					toggle_chatfield(false);
				}
		);

		send_chatline("[Welcome] Welcome to MapleStory!!", LineType::YELLOW);

		update_screen(
				Constants::Constants::get().get_viewwidth(),
				Constants::Constants::get().get_viewheight()
		);
	}

	void UIChatBar::draw(float inter) const
	{
		const Rectangle<int16_t> &strip = UIStatusBar::CHAT_STRIP;
		Point<int16_t> strip_pos = position + strip.get_left_top();

		backgrnd.draw(position, strip);
		backgrnd2.draw(position, strip);

		auto &graphics = GraphicsGL::get();

		if (chatfieldopen)
		{
			chattarget.draw(position + Point<int16_t>(2, 8));
			chattarget_label.draw(position + Point<int16_t>(6, 5));
			chatfield.draw(position);
		} else
		{
			graphics.drawrectangle(strip_pos.x(), strip_pos.y(), STRIP_WIDTH, strip.height(), 0.0f, 0.0f, 0.0f,
								   BAND_ALPHA);

			if (rowtexts.count(rowmax))
				rowtexts.at(rowmax).draw(position + Point<int16_t>(3, 5));
		}

		if (chatopen)
		{
			for (int16_t i = 0; i < CHATROWS; i++)
			{
				int16_t rowid = rowpos - i;

				if (!rowtexts.count(rowid))
					break;

				int16_t top = 1 - CHATROWHEIGHT * (i + 1);

				graphics.drawrectangle(position.x(), position.y() + top, STRIP_WIDTH, CHATROWHEIGHT, 0.0f, 0.0f, 0.0f,
									   BAND_ALPHA);
				rowtexts.at(rowid).draw(position + Point<int16_t>(3, top - 7));
			}

			Point<int16_t> prev = get_arrow_pos(Arrow::PREV);
			Point<int16_t> next = get_arrow_pos(Arrow::NEXT);
			int16_t track = next.y() - prev.y() - CHATROWHEIGHT;

			if (track > 0)
				scrollbase.draw(DrawArgument(position + prev + Point<int16_t>(0, CHATROWHEIGHT),
											 Point<int16_t>(0, track)));
		}

		bool can_scroll[2] = {
			chatopen && rowpos >= CHATROWS && rowtexts.count(rowpos - CHATROWS),
			chatopen && rowpos < rowmax
		};

		for (auto arrow : { Arrow::PREV, Arrow::NEXT })
		{
			Point<int16_t> pos = position + get_arrow_pos(arrow);

			if (can_scroll[arrow])
				arrows[arrow][arrow_state[arrow]].draw(pos);
			else
				arrows_disabled[arrow].draw(pos);
		}

		UIElement::draw_buttons(inter);
	}

	void UIChatBar::update()
	{
		UIElement::update();

		chatfield.update(position);

		for (auto iter : message_cooldowns)
			iter.second -= Constants::TIMESTEP;
	}

	void UIChatBar::update_screen(int16_t, int16_t new_height)
	{
		position = Point<int16_t>(0, new_height - UIStatusBar::HEIGHT);
	}

	void UIChatBar::send_key(int32_t keycode, bool pressed, bool escape)
	{
		if (pressed)
		{
			if (keycode == KeyAction::Id::RETURN)
				toggle_chatfield();
			else if (escape)
				toggle_chatfield(false);
		}
	}

	bool UIChatBar::is_in_range(Point<int16_t> cursorpos) const
	{
		Point<int16_t> pos = cursorpos - position;

		if (UIStatusBar::CHAT_STRIP.contains(pos))
			return true;

		return chatopen && Rectangle<int16_t>(0, STRIP_WIDTH + 15, get_logtop(), 0).contains(pos);
	}

	Cursor::State UIChatBar::send_cursor(bool clicking, Point<int16_t> cursorpos)
	{
		if (chatfieldopen)
			if (Cursor::State new_state = chatfield.send_cursor(cursorpos, clicking))
				return new_state;

		for (auto arrow : { Arrow::PREV, Arrow::NEXT })
		{
			if (get_arrow_bounds(arrow).contains(cursorpos))
			{
				if (clicking && arrow_state[arrow] != 2)
					scroll(arrow == Arrow::PREV ? -1 : 1);

				arrow_state[arrow] = clicking ? 2 : 1;

				return clicking ? Cursor::State::CLICKING : Cursor::State::CANCLICK;
			}

			arrow_state[arrow] = 0;
		}

		return UIElement::send_cursor(clicking, cursorpos);
	}

	void UIChatBar::send_scroll(double yoffset)
	{
		if (chatopen)
			scroll(yoffset > 0 ? -1 : 1);
	}

	void UIChatBar::remove_cursor()
	{
		UIElement::remove_cursor();

		arrow_state[Arrow::PREV] = 0;
		arrow_state[Arrow::NEXT] = 0;
	}

	UIElement::Type UIChatBar::get_type() const
	{
		return TYPE;
	}

	void UIChatBar::send_chatline(const std::string &line, LineType type)
	{
		rowmax++;
		rowpos = rowmax;

		Color::Name color;

		switch (type)
		{
			case LineType::RED:
				color = Color::Name::DARKRED;
				break;
			case LineType::BLUE:
				color = Color::Name::MEDIUMBLUE;
				break;
			case LineType::YELLOW:
				color = Color::Name::YELLOW;
				break;
			case LineType::GREY:
				color = Color::Name::SILVER;
				break;
			case LineType::PARTY:
				color = Color::Name::PARTYPINK;
				break;
			case LineType::BUDDY:
				color = Color::Name::ORANGE;
				break;
			default:
				color = Color::Name::WHITE;
				break;
		}

		rowtexts.emplace(
				std::piecewise_construct,
				std::forward_as_tuple(rowmax),
				std::forward_as_tuple(Text::Font::A12M, Text::Alignment::LEFT, color, line)
		);
	}

	void UIChatBar::display_message(Messages::Type line, UIChatBar::LineType type)
	{
		if (message_cooldowns[line] > 0)
			return;

		std::string message = Messages::messages[line];
		send_chatline(message, type);

		message_cooldowns[line] = MESSAGE_COOLDOWN;
	}

	void UIChatBar::toggle_chat()
	{
		toggle_chat(!chatopen);
	}

	void UIChatBar::toggle_chat(bool chat_open)
	{
		chatopen = chat_open;

		buttons[Buttons::BT_OPENCHAT]->set_active(!chatopen);
		buttons[Buttons::BT_CLOSECHAT]->set_active(chatopen);

		Setting<Chatopen>::get().save(chatopen);
	}

	void UIChatBar::toggle_chatfield()
	{
		toggle_chatfield(!chatfieldopen);
	}

	void UIChatBar::toggle_chatfield(bool chatfield_open)
	{
		chatfieldopen = chatfield_open;

		if (chatfieldopen)
		{
			chatfield.set_state(Textfield::State::FOCUSED);
		} else
		{
			chatfield.set_state(Textfield::State::DISABLED);
			chatfield.change_text("");
		}
	}

	bool UIChatBar::is_chatopen()
	{
		return chatopen;
	}

	bool UIChatBar::is_chatfieldopen()
	{
		return chatfieldopen;
	}

	void UIChatBar::set_target(Target new_target)
	{
		chat_target = new_target;

		switch (chat_target)
		{
			case Target::PARTY:
				chattarget_label.change_text("To Party");
				break;
			case Target::BUDDY:
				chattarget_label.change_text("To Buddy");
				break;
			default:
				chattarget_label.change_text("To All");
				break;
		}
		toggle_chatfield(true);
	}

	void UIChatBar::start_whisper(const std::string& name)
	{
		toggle_chatfield(true);
		chatfield.change_text("/w " + name + " ");
	}

	void UIChatBar::send_party_chat(const std::string& message)
	{
		Player& player = Stage::get().get_player();
		const Party& party = player.get_party();

		if (!party.exists())
		{
			send_chatline("You are not in a party.", LineType::RED);
			return;
		}

		std::vector<int32_t> recipients;

		for (auto& member : party.get_members())
			if (member.id != player.get_oid())
				recipients.push_back(member.id);

		MultiChatPacket(MultiChatPacket::Mode::PARTY, recipients, message).dispatch();

		// The server sends the line to the other members only.
		send_chatline(player.get_stats().get_name() + " : " + message, LineType::PARTY);
	}

	void UIChatBar::send_buddy_chat(const std::string& message)
	{
		Player& player = Stage::get().get_player();
		std::vector<int32_t> recipients = player.get_buddies().online_ids();

		if (recipients.empty())
		{
			send_chatline("None of your buddies are online.", LineType::RED);
			return;
		}

		MultiChatPacket(MultiChatPacket::Mode::BUDDY, recipients, message).dispatch();

		// As with party chat, the server sends the line to the others only.
		send_chatline(player.get_stats().get_name() + " : " + message, LineType::BUDDY);
	}

	Button::State UIChatBar::button_pressed(uint16_t buttonid)
	{
		switch (buttonid)
		{
			case Buttons::BT_OPENCHAT:
			case Buttons::BT_CLOSECHAT:
				toggle_chat();
				break;
		}

		return Button::State::NORMAL;
	}

	void UIChatBar::scroll(int16_t rows)
	{
		int16_t lowest = std::min<int16_t>(CHATROWS - 1, rowmax);

		rowpos = std::max<int16_t>(lowest, std::min<int16_t>(rowmax, rowpos + rows));
	}

	int16_t UIChatBar::get_logtop() const
	{
		return 1 - CHATROWHEIGHT * CHATROWS;
	}

	Point<int16_t> UIChatBar::get_arrow_pos(Arrow arrow) const
	{
		// The arrows run down the side of the log, or sit in the strip while
		// the log is collapsed.
		if (chatopen)
			return Point<int16_t>(STRIP_WIDTH, arrow == Arrow::PREV ? get_logtop() : 1 - CHATROWHEIGHT);

		return Point<int16_t>(STRIP_WIDTH, arrow == Arrow::PREV ? 6 : 6 + CHATROWHEIGHT);
	}

	Rectangle<int16_t> UIChatBar::get_arrow_bounds(Arrow arrow) const
	{
		Point<int16_t> lt = position + get_arrow_pos(arrow);

		return Rectangle<int16_t>(lt, lt + arrows[arrow][0].get_dimensions());
	}
}
