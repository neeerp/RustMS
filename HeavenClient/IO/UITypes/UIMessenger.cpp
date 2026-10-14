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
#include "UIMessenger.h"

#include "UINotice.h"

#include "../UI.h"

#include "../Components/AreaButton.h"
#include "../Components/MapleButton.h"

#include "../../Constants.h"
#include "../../Gameplay/Stage.h"
#include "../../Net/Packets/MessagingPackets.h"
#include "../../Util/NxFiles.h"

#include <algorithm>
#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// Measured from the v83 art (UIWindow.img/Messenger/backgrnd).
		constexpr Point<int16_t> BT_MIN = Point<int16_t>(262, 5);
		constexpr Point<int16_t> BT_CLOSE = Point<int16_t>(277, 5);
		constexpr Point<int16_t> BT_ENTER = Point<int16_t>(236, 317);

		// The title-only window shown while minimized (backgrnd3).
		constexpr Point<int16_t> MIN_BT_MAX = Point<int16_t>(149, 5);
		constexpr Point<int16_t> MIN_BT_CLOSE = Point<int16_t>(164, 5);
		constexpr Point<int16_t> MIN_BLINK = Point<int16_t>(135, 6);
		constexpr uint16_t BLINK_PERIOD = 1000;

		// Each member stands on a shadow with their name bar below it.
		constexpr int16_t SLOT_X[UIMessenger::SLOTS] = { 53, 147, 239 };
		constexpr int16_t FEET_Y = 119;
		constexpr int16_t SLOT_TOP = 25;
		constexpr int16_t NAMEBAR_Y = 124;
		constexpr int16_t NAME_Y = 121;
		constexpr Point<int16_t> BALLOON = Point<int16_t>(8, -88);
		constexpr uint16_t BALLOON_TIME = 3000;

		// The chat log and its scroll bar.
		constexpr int16_t LOG_LEFT = 10;
		constexpr int16_t LOG_TOP = 152;
		constexpr int16_t LOG_BOTTOM = 300;
		constexpr int16_t LOG_WIDTH = 250;
		constexpr int16_t LOG_ROW = 13;
		constexpr size_t LOG_LIMIT = 200;
		constexpr int16_t SLIDER_X = 272;
		const Range<int16_t> SLIDER_RANGE = Range<int16_t>(150, 289);

		constexpr size_t INPUT_LIMIT = 70;
		constexpr size_t NAME_LIMIT = 12;

		const std::string INVITE_COMMAND = "/invite ";
		const std::string QUIT_COMMAND = "/q";

		// The help the window opens with, as in period footage.
		const std::string HELP[] = {
			"[ Maple Messenger Help ]",
			"-----------------------------",
			"Invite : /invite character-name",
			"End : /q",
			"-----------------------------"
		};
	}

	UIMessenger::UIMessenger() : UIDragElement<PosMESSENGER>(), own_slot(0), log_height(0), log_row(0),
			minimized(false), unread(false), blink_elapsed(0)
	{
		nl::node Messenger = NxFiles::ui_classic["UIWindow.img"]["Messenger"];
		nl::node Basic = NxFiles::ui_classic["Basic.img"];

		backgrnd = Messenger["backgrnd"];
		minimized_backgrnd = Messenger["backgrnd3"];
		blink = Messenger["blink"];

		for (size_t i = 0; i < namebars.size(); i++)
			namebars[i] = Messenger["NameBar"][std::to_string(i)];

		buttons[Buttons::MIN] = std::make_unique<MapleButton>(Basic["BtMin"], BT_MIN);
		buttons[Buttons::MAX] = std::make_unique<MapleButton>(Basic["BtMax"], MIN_BT_MAX);
		buttons[Buttons::CLOSE] = std::make_unique<MapleButton>(Basic["BtClose"], BT_CLOSE);
		buttons[Buttons::ENTER] = std::make_unique<MapleButton>(Messenger["BtEnter"], BT_ENTER);

		Point<int16_t> namebar = namebars[0].get_dimensions();

		for (size_t i = 0; i < SLOTS; i++)
		{
			Point<int16_t> slot = Point<int16_t>(SLOT_X[i] - namebar.x() / 2, SLOT_TOP);
			Point<int16_t> area = Point<int16_t>(namebar.x(), NAMEBAR_Y + namebar.y() - SLOT_TOP);

			buttons[Buttons::SLOT + i] = std::make_unique<AreaButton>(slot, area);

			members[i].name = Text(Text::Font::A11M, Text::Alignment::CENTER, Color::Name::WHITE);
			members[i].balloon = Messenger["chatBalloon"];
		}

		slider = Slider(
				Slider::Type::CLASSIC, SLIDER_RANGE, SLIDER_X, log_rows(), 0,
				[&](bool upwards)
				{
					int16_t total = (log_height + LOG_ROW - 1) / LOG_ROW;
					int16_t shift = upwards ? -1 : 1;

					if (log_row + shift >= 0 && log_row + shift + log_rows() <= total)
						log_row += shift;
				}
		);

		input = Textfield(Text::Font::A11M, Text::Alignment::LEFT, Color::Name::BLACK,
						  Rectangle<int16_t>(12, 218, 317, 333), INPUT_LIMIT);

		input.set_enter_callback(
				[&](std::string text)
				{
					input.change_text("");
					submit(text);
				}
		);

		input.set_key_callback(
				KeyAction::Id::ESCAPE,
				[&]()
				{
					input.set_state(Textfield::State::DISABLED);
					input.set_state(Textfield::State::NORMAL);
				}
		);

		set_minimized(false);
	}

	void UIMessenger::draw(float inter) const
	{
		if (minimized)
		{
			minimized_backgrnd.draw(position);

			if (unread && blink_elapsed % BLINK_PERIOD < BLINK_PERIOD / 2)
				blink.draw(position + MIN_BLINK);

			UIElement::draw_buttons(inter);

			return;
		}

		backgrnd.draw(position);

		Point<int16_t> namebar = namebars[0].get_dimensions();

		for (size_t i = 0; i < SLOTS; i++)
		{
			const Member &member = members[i];
			Point<int16_t> feet = position + Point<int16_t>(SLOT_X[i], FEET_Y);
			Point<int16_t> bar = position + Point<int16_t>(SLOT_X[i] - namebar.x() / 2, NAMEBAR_Y);

			if (!member.present)
			{
				namebars[0].draw(bar);
				continue;
			}

			// Our own avatar follows our equipment as it changes.
			const CharLook &look = i == own_slot ? Stage::get().get_player().get_look() : member.look;

			look.draw(feet, false, Stance::Id::STAND1, Expression::Id::DEFAULT);
			namebars[i + 1].draw(bar);
			member.name.draw(position + Point<int16_t>(SLOT_X[i], NAME_Y));

			if (member.balloon_left > 0)
				member.balloon.draw(feet + BALLOON, inter);
		}

		Range<int16_t> clip = Range<int16_t>(position.y() + LOG_TOP, position.y() + LOG_BOTTOM);
		int16_t y = LOG_TOP - log_row * LOG_ROW;

		for (const Text &line : log)
		{
			if (y + line.height() > LOG_TOP && y < LOG_BOTTOM)
				line.draw(DrawArgument(position + Point<int16_t>(LOG_LEFT, y)), clip);

			y += line.height();
		}

		slider.draw(position);
		input.draw(position);

		UIElement::draw_buttons(inter);
	}

	void UIMessenger::update()
	{
		UIElement::update();

		input.update(position);

		blink_elapsed = (blink_elapsed + Constants::TIMESTEP) % BLINK_PERIOD;

		for (Member &member : members)
		{
			if (member.balloon_left == 0)
				continue;

			member.balloon.update();
			member.balloon_left -= std::min(member.balloon_left, Constants::TIMESTEP);
		}
	}

	Cursor::State UIMessenger::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		Cursor::State dstate = UIDragElement::send_cursor(clicked, cursorpos);

		if (dragged || minimized)
			return dstate;

		if (Cursor::State sstate = slider.send_cursor(cursorpos - position, clicked))
			return sstate;

		Cursor::State tstate = input.send_cursor(cursorpos, clicked);

		if (tstate != Cursor::State::IDLE)
			return tstate;

		return dstate;
	}

	void UIMessenger::send_scroll(double yoffset)
	{
		if (!minimized)
			slider.send_scroll(yoffset);
	}

	void UIMessenger::send_key(int32_t, bool, bool)
	{
		// Closing the window leaves the messenger, so escape keeps it open.
	}

	UIElement::Type UIMessenger::get_type() const
	{
		return TYPE;
	}

	void UIMessenger::toggle()
	{
		auto messenger = UI::get().get_element<UIMessenger>();

		if (messenger && messenger->is_active())
			messenger->set_minimized(!messenger->minimized);
		else
			MessengerOpenPacket(0).dispatch();
	}

	void UIMessenger::seat(uint8_t slot)
	{
		auto messenger = UI::get().get_element<UIMessenger>();

		if (!messenger)
			messenger = UI::get().emplace<UIMessenger>();
		else if (!messenger->is_active())
			messenger->makeactive();

		if (messenger)
			messenger->join(slot);
	}

	void UIMessenger::set_member(uint8_t slot, const CharLook &look, const std::string &name)
	{
		if (slot >= SLOTS)
			return;

		Member &member = members[slot];
		member.present = true;
		member.look = look;
		member.name.change_text(name);
		buttons[Buttons::SLOT + slot]->set_active(false);
	}

	void UIMessenger::remove_member(uint8_t slot)
	{
		if (slot >= SLOTS || slot == own_slot)
			return;

		clear_member(slot);
	}

	void UIMessenger::add_chat(const std::string &line)
	{
		for (size_t i = 0; i < SLOTS; i++)
		{
			Member &member = members[i];

			if (!member.present || line.rfind(member.name.get_text() + " : ", 0) != 0)
				continue;

			member.balloon.reset();
			member.balloon_left = BALLOON_TIME;
			break;
		}

		if (minimized)
			unread = true;

		add_line(line);
	}

	void UIMessenger::add_note(const std::string &note)
	{
		add_line(note);
	}

	Button::State UIMessenger::button_pressed(uint16_t buttonid)
	{
		switch (buttonid)
		{
			case Buttons::MIN:
				set_minimized(true);
				break;
			case Buttons::MAX:
				set_minimized(false);
				break;
			case Buttons::CLOSE:
				UI::get().emplace<UIYesNo>(
						"Will you exit the chat room?",
						[](bool yes)
						{
							if (!yes)
								return;

							if (auto messenger = UI::get().get_element<UIMessenger>())
								messenger->leave();
						}
				);
				break;
			case Buttons::ENTER:
			{
				std::string text = input.get_text();
				input.change_text("");
				submit(text);
				break;
			}
			default:
				ask_invite();
				break;
		}

		return Button::State::NORMAL;
	}

	void UIMessenger::join(uint8_t slot)
	{
		if (slot >= SLOTS)
			return;

		for (size_t i = 0; i < SLOTS; i++)
			clear_member(i);

		own_slot = slot;
		set_member(slot, CharLook(), Stage::get().get_player().get_name());

		log.clear();
		log_height = 0;

		for (const std::string &line : HELP)
			add_line(line);

		unread = false;
		set_minimized(false);
	}

	void UIMessenger::clear_member(uint8_t slot)
	{
		Member &member = members[slot];
		member.present = false;
		member.look = CharLook();
		member.name.change_text("");
		member.balloon_left = 0;
		buttons[Buttons::SLOT + slot]->set_active(!minimized);
	}

	void UIMessenger::leave()
	{
		MessengerLeavePacket().dispatch();

		if (input.get_state() == Textfield::State::FOCUSED)
			input.set_state(Textfield::State::DISABLED);

		input.set_state(Textfield::State::NORMAL);
		input.change_text("");

		deactivate();
	}

	void UIMessenger::submit(const std::string &text)
	{
		if (text.empty())
			return;

		if (text == QUIT_COMMAND)
		{
			leave();
		} else if (text.rfind(INVITE_COMMAND, 0) == 0)
		{
			invite(text.substr(INVITE_COMMAND.size()));
		} else
		{
			std::string line = Stage::get().get_player().get_name() + " : " + text;

			MessengerChatPacket(line).dispatch();
			add_chat(line);
		}
	}

	void UIMessenger::ask_invite()
	{
		UI::get().emplace<UIEnterText>(
				"Whom would you like to invite?",
				[](const std::string &name)
				{
					if (auto messenger = UI::get().get_element<UIMessenger>())
						messenger->invite(name);
				},
				NAME_LIMIT
		);
	}

	void UIMessenger::invite(const std::string &name)
	{
		if (name.empty() || name.size() > NAME_LIMIT)
			return;

		MessengerInvitePacket(name).dispatch();
	}

	void UIMessenger::add_line(const std::string &line)
	{
		int16_t total = (log_height + LOG_ROW - 1) / LOG_ROW;
		bool at_bottom = log_row + log_rows() >= total;

		log.emplace_back(Text::Font::A11M, Text::Alignment::LEFT, Color::Name::BLACK, line, LOG_WIDTH);
		log_height += log.back().height();

		if (log.size() > LOG_LIMIT)
		{
			log_height -= log.front().height();
			log.erase(log.begin());
		}

		if (at_bottom)
			scroll_to_bottom();
		else
			slider.setrows(log_row, log_rows(), (log_height + LOG_ROW - 1) / LOG_ROW);
	}

	void UIMessenger::scroll_to_bottom()
	{
		int16_t total = (log_height + LOG_ROW - 1) / LOG_ROW;

		log_row = std::max<int16_t>(0, total - log_rows());
		slider.setrows(log_row, log_rows(), total);
	}

	void UIMessenger::set_minimized(bool min)
	{
		minimized = min;

		if (minimized)
		{
			dimension = minimized_backgrnd.get_dimensions();

			if (input.get_state() == Textfield::State::FOCUSED)
			{
				input.set_state(Textfield::State::DISABLED);
				input.set_state(Textfield::State::NORMAL);
			}
		} else
		{
			dimension = backgrnd.get_dimensions();
			unread = false;
		}

		dragarea = Point<int16_t>(dimension.x(), 20);

		buttons[Buttons::MIN]->set_active(!minimized);
		buttons[Buttons::MAX]->set_active(minimized);
		buttons[Buttons::ENTER]->set_active(!minimized);
		buttons[Buttons::CLOSE]->set_position(minimized ? MIN_BT_CLOSE : BT_CLOSE);

		for (size_t i = 0; i < SLOTS; i++)
			buttons[Buttons::SLOT + i]->set_active(!minimized && !members[i].present);
	}

	int16_t UIMessenger::log_rows() const
	{
		return (LOG_BOTTOM - LOG_TOP) / LOG_ROW;
	}
}
