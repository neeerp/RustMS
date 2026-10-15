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

		// Whether input is /find, with the name it looks for, if any.
		bool parse_find_command(const std::string& input, std::string& target)
		{
			static const std::string COMMAND = "/find";

			if (input != COMMAND && input.rfind(COMMAND + " ", 0) != 0)
				return false;

			size_t target_start = input.find_first_not_of(' ', COMMAND.length());

			if (target_start != std::string::npos)
				target = input.substr(target_start, input.find(' ', target_start) - target_start);

			return true;
		}

		// The original fills the whole log with black at half opacity.
		constexpr float LOG_ALPHA = 0.5f;

		// The input field, from the status bar's top left
		constexpr Rectangle<int16_t> CHATFIELD = Rectangle<int16_t>(87, 530, 5, 25);

		// The original places a line by the top of its text; text here is
		// placed this far above that.
		constexpr int16_t TEXT_RAISE = 7;
	}

	UIChatBar::UIChatBar()
	{
		chatopen = Setting<Chatopen>::get().load();
		chatfieldopen = false;
		lastpos = 0;
		rowpos = 0;
		rowmax = -1;
		arrow_pressed[Arrow::PREV] = false;
		arrow_pressed[Arrow::NEXT] = false;
		scrollbar_pressed = false;
		thumb_dragging = false;
		drag_y = 0;
		drag_pos = 0;

		nl::node base = NxFiles::ui_classic["StatusBar.img"]["base"];
		nl::node basic = NxFiles::ui_classic["Basic.img"];
		nl::node scrollbar = basic["VScr5"];

		backgrnd = base["backgrnd"];
		backgrnd2 = base["backgrnd2"];
		chattarget = base["chatTarget"];
		chattarget_label = Text(Text::Font::A11M, Text::Alignment::LEFT, Color::Name::WHITE, "To All");
		chat_target = Target::ALL;

		// The original's scroll bar has only a normal and a pressed look.
		for (size_t i = 0; i < 2; i++)
		{
			arrows[Arrow::PREV][i] = scrollbar["enabled"]["prev" + std::to_string(i)];
			arrows[Arrow::NEXT][i] = scrollbar["enabled"]["next" + std::to_string(i)];
			thumb[i] = scrollbar["enabled"]["thumb" + std::to_string(i)];
		}

		arrows_disabled[Arrow::PREV] = scrollbar["disabled"]["prev"];
		arrows_disabled[Arrow::NEXT] = scrollbar["disabled"]["next"];
		scrollbase = scrollbar["enabled"]["base"];
		scrollbase_disabled = scrollbar["disabled"]["base"];

		buttons[Buttons::BT_OPENCHAT] = std::make_unique<MapleButton>(basic["BtMax"], Point<int16_t>(536, 12));
		buttons[Buttons::BT_CLOSECHAT] = std::make_unique<MapleButton>(basic["BtMin"], Point<int16_t>(536, 12));
		buttons[Buttons::BT_OPENCHAT]->set_active(!chatopen);
		buttons[Buttons::BT_CLOSECHAT]->set_active(chatopen);

		chatfield = Textfield(Text::A12M, Text::LEFT, Color::Name::BLACK,
							  CHATFIELD, 0);
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
							else if (parse_find_command(msg, target))
							{
								if (target.empty())
									send_chatline("Usage: /find <name>", LineType::RED);
								else
									FindPacket(target).dispatch();
							}
							else if (chat_target == Target::PARTY)
							{
								send_party_chat(msg);
							}
							else if (chat_target == Target::BUDDY)
							{
								send_buddy_chat(msg);
							}
							// Commands the client doesn't know aren't chat.
							else if (msg[0] != '/')
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

		backgrnd.draw(position, strip);
		backgrnd2.draw(position, strip);

		auto &graphics = GraphicsGL::get();

		if (chatfieldopen || chatopen)
		{
			chattarget.draw(position + Point<int16_t>(2, 8));
			chattarget_label.draw(position + Point<int16_t>(6, 5));
			chatfield.draw(position);
		}

		int16_t sbwidth = arrows[Arrow::PREV][0].width();
		int16_t logtop = get_logtop();
		int16_t logheight = get_logheight();
		int16_t logbottom = logtop + logheight;

		graphics.drawrectangle(position.x(), position.y() + logtop, LOG_WIDTH - sbwidth, logheight, 0.0f, 0.0f, 0.0f,
							   LOG_ALPHA);

		for (int16_t i = 0; i < get_logrows(); i++)
		{
			auto row = rowtexts.find(rowpos - i);

			if (row == rowtexts.end())
				break;

			int16_t top = logbottom - CHATROWHEIGHT * (i + 1);

			row->second.draw(position + Point<int16_t>(3, top - TEXT_RAISE));
		}

		Point<int16_t> bar = position + get_scrollbar_pos();
		int16_t length = get_scrollbar_length();
		int16_t button = arrows[Arrow::PREV][0].height();
		int16_t track = length - 2 * button;
		bool enabled = get_scrollrange() >= 2;

		if (track > 0)
			(enabled ? scrollbase : scrollbase_disabled).draw(DrawArgument(bar + Point<int16_t>(0, button),
																		   Point<int16_t>(0, track)));

		for (auto arrow : { Arrow::PREV, Arrow::NEXT })
		{
			Point<int16_t> pos = bar + Point<int16_t>(0, arrow == Arrow::PREV ? 0 : length - button);

			if (enabled)
				arrows[arrow][arrow_pressed[arrow] ? 1 : 0].draw(pos);
			else
				arrows_disabled[arrow].draw(pos);
		}

		if (std::optional<int16_t> offset = get_thumb_offset(thumb[0].height()))
			thumb[thumb_dragging ? 1 : 0].draw(bar + Point<int16_t>(0, *offset));

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

		int16_t logtop = get_logtop();
		Point<int16_t> bar = get_scrollbar_pos();

		return Rectangle<int16_t>(0, LOG_WIDTH, logtop, logtop + get_logheight()).contains(pos) ||
			   Rectangle<int16_t>(bar, bar + Point<int16_t>(arrows[Arrow::PREV][0].width(),
															 get_scrollbar_length())).contains(pos);
	}

	Cursor::State UIChatBar::send_cursor(bool clicking, Point<int16_t> cursorpos)
	{
		bool pressed = clicking && !scrollbar_pressed;
		scrollbar_pressed = clicking;

		if (thumb_dragging)
		{
			if (clicking)
			{
				int16_t button = arrows[Arrow::PREV][0].height();
				int16_t travel = get_scrollbar_length() - 3 * button;

				if (travel > 0)
					set_scrollpos(drag_pos + get_scrollrange() * (cursorpos.y() - drag_y) / travel);

				return Cursor::State::VSCROLLIDLE;
			}

			thumb_dragging = false;
		}

		if (chatfieldopen)
		{
			if (Cursor::State new_state = chatfield.send_cursor(cursorpos, clicking))
				return new_state;
		} else if (chatopen && pressed && CHATFIELD.contains(cursorpos - position))
		{
			// The expanded log keeps the input field, so a click there starts
			// typing.
			toggle_chatfield(true);

			return Cursor::State::CLICKING;
		}

		Point<int16_t> bar = position + get_scrollbar_pos();
		Point<int16_t> rel = cursorpos - bar;
		int16_t width = arrows[Arrow::PREV][0].width();
		int16_t length = get_scrollbar_length();
		int16_t button = arrows[Arrow::PREV][0].height();

		arrow_pressed[Arrow::PREV] = false;
		arrow_pressed[Arrow::NEXT] = false;

		if (rel.x() < 0 || rel.x() >= width || rel.y() < 0 || rel.y() >= length)
			return UIElement::send_cursor(clicking, cursorpos);

		if (get_scrollrange() < 2)
			return Cursor::State::IDLE;

		if (rel.y() < button)
		{
			arrow_pressed[Arrow::PREV] = clicking;

			if (pressed)
				scroll(-1);
		} else if (rel.y() >= length - button)
		{
			arrow_pressed[Arrow::NEXT] = clicking;

			if (pressed)
				scroll(1);
		} else if (pressed)
		{
			// Clicking the track either side of the thumb pages through the
			// log, at least sixteen lines at a time.
			int16_t page = std::max<int16_t>(16, get_scrollrange() / 16);
			// The original only counts an arrow's height of the thumb.
			int16_t thumbtop = get_thumb_offset(button).value_or(button);

			if (rel.y() < thumbtop)
			{
				set_scrollpos(get_scrollpos() - page);
			} else if (rel.y() > thumbtop + button)
			{
				set_scrollpos(get_scrollpos() + page);
			} else
			{
				thumb_dragging = true;
				drag_y = cursorpos.y();
				drag_pos = get_scrollpos();

				return Cursor::State::VSCROLLIDLE;
			}
		}

		return clicking ? Cursor::State::CLICKING : Cursor::State::CANCLICK;
	}

	void UIChatBar::send_scroll(double yoffset)
	{
		if (chatopen)
			scroll(yoffset > 0 ? -1 : 1);
	}

	void UIChatBar::remove_cursor()
	{
		UIElement::remove_cursor();

		arrow_pressed[Arrow::PREV] = false;
		arrow_pressed[Arrow::NEXT] = false;
		scrollbar_pressed = false;
		thumb_dragging = false;
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
			case LineType::PINK:
				color = Color::Name::MONALISA;
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
		// As in the original, changing how the log shows returns it to the
		// latest line.
		rowpos = rowmax;

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

		// The collapsed log moves above the strip while typing.
		if (!chatopen)
			rowpos = rowmax;

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
		set_scrollpos(get_scrollpos() + rows);
	}

	int16_t UIChatBar::get_logrows() const
	{
		return chatopen ? CHATROWS : 1;
	}

	int16_t UIChatBar::get_logheight() const
	{
		return chatopen ? LOG_HEIGHT : LINE_LOG_HEIGHT;
	}

	// The original lays the chat out in a window 22 pixels below the top of
	// the screen, which puts the status bar 507 below that window's top. Its
	// log sits five pixels above where its mode says.
	int16_t UIChatBar::get_logtop() const
	{
		if (chatopen)
			return -3 - LOG_HEIGHT;

		return chatfieldopen ? -26 : 1;
	}

	Point<int16_t> UIChatBar::get_scrollbar_pos() const
	{
		int16_t x = 566 - arrows[Arrow::PREV][0].width();

		if (chatopen || chatfieldopen)
			return Point<int16_t>(x, 3 - get_logheight());

		return Point<int16_t>(x, 6);
	}

	int16_t UIChatBar::get_scrollbar_length() const
	{
		return chatopen ? LOG_HEIGHT - 2 : LINE_LOG_HEIGHT + 1;
	}

	int16_t UIChatBar::get_scrollrange() const
	{
		return rowmax + 1 - get_logrows() + 1;
	}

	int16_t UIChatBar::get_scrollpos() const
	{
		return std::max<int16_t>(0, rowpos - get_logrows() + 1);
	}

	void UIChatBar::set_scrollpos(int16_t pos)
	{
		pos = std::max<int16_t>(0, std::min<int16_t>(pos, get_scrollrange() - 1));
		rowpos = std::min<int16_t>(rowmax, pos + get_logrows() - 1);
	}

	std::optional<int16_t> UIChatBar::get_thumb_offset(int16_t height) const
	{
		int16_t range = get_scrollrange();
		int16_t button = arrows[Arrow::PREV][0].height();
		int16_t travel = get_scrollbar_length() - 2 * button - height;

		// The collapsed log's scroll bar is just its two arrows.
		if (range < 2 || travel < 0)
			return std::nullopt;

		return button + travel * get_scrollpos() / (range - 1);
	}
}
