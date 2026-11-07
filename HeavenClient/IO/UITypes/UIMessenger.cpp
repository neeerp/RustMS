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

#include "UIChatBar.h"
#include "UIClaim.h"
#include "UINotice.h"

#include "../UI.h"

#include "../Components/MapleButton.h"

#include "../../Configuration.h"
#include "../../Constants.h"
#include "../../Data/CurseData.h"
#include "../../Gameplay/Stage.h"
#include "../../Net/Packets/MessagingPackets.h"
#include "../../Util/NxFiles.h"

#include <algorithm>
#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// The buttons of the title bar, always the same three: y is one higher
		// on the title alone (0x850769).
		constexpr int16_t BT_MIN_X = 248;
		constexpr int16_t BT_MAX_X = 262;
		constexpr int16_t BT_CLOSE_X = 276;
		constexpr int16_t TITLE_BT_MIN_X = 138;
		constexpr int16_t TITLE_BT_MAX_X = 152;
		constexpr int16_t TITLE_BT_CLOSE_X = 166;
		constexpr int16_t BT_Y = 6;
		constexpr int16_t TITLE_BT_Y = 5;

		constexpr int16_t BT_ENTER_X = 249;
		constexpr int16_t BT_CLAIM_X = 225;

		// The title's blink (0x84e158) comes on every 900 ms (0x84eb90).
		constexpr Point<int16_t> TITLE_BLINK = Point<int16_t>(108, 6);
		constexpr uint16_t BLINK_PERIOD = 900;

		// Where the window can be dragged by.
		constexpr int16_t DRAG_HEIGHT = 50;
		constexpr int16_t TITLE_DRAG_HEIGHT = 13;

		// Each member stands on a shadow with their name bar below it.
		constexpr int16_t SLOT_X[UIMessenger::SLOTS] = { 53, 147, 239 };
		constexpr int16_t FEET_Y = 119;
		constexpr Point<int16_t> BALLOON = Point<int16_t>(-28, -99);
		constexpr float BALLOON_ALPHA = 207.0f / 255;

		// The avatar's box for the channel tooltip, around the feet (I).
		constexpr int16_t AVATAR_HALF_WIDTH = 25;
		constexpr int16_t AVATAR_HEIGHT = 70;

		// What moves between the full window and the short one (0x850000,
		// 0x850e28). The short one has no avatars.
		struct Layout
		{
			int16_t log_top;
			int16_t log_bottom;
			int16_t slider_top;
			int16_t slider_bottom;
			int16_t input_top;
			int16_t enter_y;
			int16_t claim_y;
			int16_t namebar_y;
			int16_t name_y;
		};

		constexpr Layout FULL_LAYOUT = { 149, 298, 150, 289, 314, 317, 315, 124, 121 };
		constexpr Layout SHORT_LAYOUT = { 54, 177, 57, 167, 190, 193, 191, 30, 27 };

		// The input. HeavenClient's text stands lower than v83's, so the field
		// starts 3 px above v83's and reaches the same bottom.
		constexpr int16_t INPUT_LEFT = 12;
		constexpr int16_t INPUT_RIGHT = 218;
		constexpr int16_t INPUT_HEIGHT = 19;
		constexpr size_t INPUT_LIMIT = 70;

		// The chat log and its scroll bar. Wrapped rows lose v83's one-space
		// indent to Text's own wrapping.
		constexpr int16_t LOG_LEFT = 14;
		constexpr int16_t LOG_WIDTH = 240;
		constexpr int16_t LOG_ROW = 13;
		constexpr int16_t LOG_ROW_ADJUST = 4;
		// A row's letters start this far below where it is drawn: above it, only the row before shows.
		constexpr int16_t LOG_CLIP_TOP = 5;
		constexpr size_t LOG_LIMIT = 200;
		constexpr int16_t SLIDER_X = 272;

		// How far a line of the log reaches: v83's rows stand 13 px apart
		// (0x850e9a), closer than this font's own line.
		int16_t rows_high(const Text &line)
		{
			return line.height() - LOG_ROW_ADJUST;
		}

		// Each seat's lines have its colour (0x84ec53): v83 fonts 11, 6, 13.
		constexpr Color::Name SEAT_COLORS[UIMessenger::SLOTS] = {
			Color::Name::SEATBLUE,
			Color::Name::BRIGHTRED,
			Color::Name::DEEPGREEN
		};

		// Who is typing, on the window's dark footer (0x850ec9).
		constexpr int16_t TYPING_ICON_X = 10;
		constexpr int16_t TYPING_TEXT_X = 26;
		constexpr int16_t TYPING_FROM_BOTTOM = 20;
		constexpr int16_t TYPING_TEXT_RAISE = 3;

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
			size(Size::FULL), unread(false), blink_elapsed(0), channel_tip(false)
	{
		nl::node Messenger = NxFiles::ui_classic["UIWindow.img"]["Messenger"];
		nl::node Basic = NxFiles::ui_classic["Basic.img"];

		backgrnd = Messenger["backgrnd"];
		short_backgrnd = Messenger["backgrnd2"];
		title_backgrnd = Messenger["backgrnd3"];
		blink = Messenger["blink"];

		for (size_t i = 0; i < namebars.size(); i++)
			namebars[i] = Messenger["NameBar"][std::to_string(i)];

		buttons[Buttons::MIN] = std::make_unique<MapleButton>(Basic["BtMin"], Point<int16_t>(BT_MIN_X, BT_Y));
		buttons[Buttons::MAX] = std::make_unique<MapleButton>(Basic["BtMax"], Point<int16_t>(BT_MAX_X, BT_Y));
		buttons[Buttons::CLOSE] = std::make_unique<MapleButton>(Basic["BtClose"], Point<int16_t>(BT_CLOSE_X, BT_Y));
		buttons[Buttons::ENTER] = std::make_unique<MapleButton>(Messenger["BtEnter"], Point<int16_t>(BT_ENTER_X, FULL_LAYOUT.enter_y));
		buttons[Buttons::CLAIM] = std::make_unique<MapleButton>(Basic["BtClaim"], Point<int16_t>(BT_CLAIM_X, FULL_LAYOUT.claim_y));

		for (size_t i = 0; i < SLOTS; i++)
		{
			members[i].name = Text(Text::Font::A11M, Text::Alignment::CENTER, Color::Name::WHITE);
			members[i].balloon = Messenger["chatBalloon"];
		}

		typing_row = Text(Text::Font::A11M, Text::Alignment::LEFT, Color::Name::WHITE);

		slider = Slider(
				Slider::Type::CLASSIC, Range<int16_t>(FULL_LAYOUT.slider_top, FULL_LAYOUT.slider_bottom), SLIDER_X, log_rows(), 0,
				[&](bool upwards)
				{
					int16_t total = (log_height + LOG_ROW - 1) / LOG_ROW;
					int16_t shift = upwards ? -1 : 1;

					if (log_row + shift >= 0 && log_row + shift + log_rows() <= total)
						log_row += shift;
				}
		);

		input = Textfield(Text::Font::A11M, Text::Alignment::LEFT, Color::Name::BLACK,
						  Rectangle<int16_t>(INPUT_LEFT, INPUT_RIGHT, FULL_LAYOUT.input_top, FULL_LAYOUT.input_top + INPUT_HEIGHT),
						  INPUT_LIMIT);

		input.set_enter_callback([&](std::string) { submit(); });

		input.set_key_callback(KeyAction::Id::ESCAPE, [&]() { ask_leave(); });

		// With nothing in the input, left and right let go of it.
		input.set_key_callback(KeyAction::Id::LEFT, [&]() { drop_focus(); });
		input.set_key_callback(KeyAction::Id::RIGHT, [&]() { drop_focus(); });

		input.set_key_callback(
				KeyAction::Id::UP,
				[&]()
				{
					std::string line;

					if (history.older(line))
						input.change_text(line);
				}
		);

		input.set_key_callback(
				KeyAction::Id::DOWN,
				[&]()
				{
					std::string line;

					if (history.newer(line))
						input.change_text(line);
				}
		);

		set_size(Size::FULL);
	}

	void UIMessenger::draw(float inter) const
	{
		if (size == Size::TITLE)
		{
			title_backgrnd.draw(position);

			if (unread && blink_elapsed < BLINK_PERIOD / 2)
				blink.draw(position + TITLE_BLINK);

			UIElement::draw_buttons(inter);

			return;
		}

		const Layout &at = size == Size::FULL ? FULL_LAYOUT : SHORT_LAYOUT;

		(size == Size::FULL ? backgrnd : short_backgrnd).draw(position);

		Point<int16_t> namebar = namebars[0].get_dimensions();

		for (size_t i = 0; i < SLOTS; i++)
		{
			const Member &member = members[i];
			Point<int16_t> feet = position + Point<int16_t>(SLOT_X[i], FEET_Y);
			Point<int16_t> bar = position + Point<int16_t>(SLOT_X[i] - namebar.x() / 2, at.namebar_y);

			if (!member.present)
			{
				namebars[0].draw(bar);
				continue;
			}

			if (size == Size::FULL)
			{
				// Our own avatar follows our equipment as it changes.
				const CharLook &look = i == own_slot ? Stage::get().get_player().get_look() : member.look;

				look.draw(feet, false, Stance::Id::STAND1, Expression::Id::DEFAULT);
			}

			namebars[i + 1].draw(bar);
			member.name.draw(position + Point<int16_t>(SLOT_X[i], at.name_y));

			if (member.balloon_shown)
				member.balloon.draw(DrawArgument(feet + BALLOON, BALLOON_ALPHA), inter);
		}

		Range<int16_t> clip = Range<int16_t>(position.y() + at.log_top + LOG_CLIP_TOP, position.y() + at.log_bottom);
		int16_t y = at.log_top - log_row * LOG_ROW;

		for (const Text &line : log)
		{
			if (y + line.height() > at.log_top && y < at.log_top + log_rows() * LOG_ROW)
				line.draw(DrawArgument(position + Point<int16_t>(LOG_LEFT, y)), clip);

			y += rows_high(line);
		}

		if (!typing_row.get_text().empty())
		{
			int16_t row_y = dimension.y() - TYPING_FROM_BOTTOM;

			blink.draw(position + Point<int16_t>(TYPING_ICON_X, row_y));
			typing_row.draw(position + Point<int16_t>(TYPING_TEXT_X, row_y - TYPING_TEXT_RAISE));
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
			if (member.balloon_shown)
				member.balloon.update();

		std::string own = Stage::get().get_player().get_name();

		switch (typing_gate.step(!input.get_text().empty(), Constants::TIMESTEP))
		{
			case messenger_rules::TypingGate::Notice::STARTED:
				MessengerChatPacket(own + "1").dispatch();
				buttons[Buttons::ENTER]->set_state(Button::State::NORMAL);
				break;
			case messenger_rules::TypingGate::Notice::STOPPED:
				MessengerChatPacket(own + "0").dispatch();
				buttons[Buttons::ENTER]->set_state(Button::State::DISABLED);
				break;
			case messenger_rules::TypingGate::Notice::NONE:
				break;
		}
	}

	Cursor::State UIMessenger::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		Cursor::State dstate = UIDragElement::send_cursor(clicked, cursorpos);

		if (dragged || size == Size::TITLE)
		{
			clear_channel();

			return dstate;
		}

		show_channel(cursorpos);

		if (Cursor::State sstate = slider.send_cursor(cursorpos - position, clicked))
			return sstate;

		Cursor::State tstate = input.send_cursor(cursorpos, clicked);

		if (tstate != Cursor::State::IDLE)
			return tstate;

		return dstate;
	}

	void UIMessenger::send_scroll(double yoffset)
	{
		if (size != Size::TITLE)
			slider.send_scroll(yoffset);
	}

	void UIMessenger::send_key(int32_t, bool, bool)
	{
		// Closing the window leaves the messenger, so escape keeps it open.
	}

	void UIMessenger::remove_cursor()
	{
		UIDragElement::remove_cursor();

		clear_channel();
	}

	UIElement::Type UIMessenger::get_type() const
	{
		return TYPE;
	}

	void UIMessenger::toggle()
	{
		auto messenger = UI::get().get_element<UIMessenger>();

		if (messenger && messenger->is_active())
			messenger->set_size(messenger->size == Size::TITLE ? Size::FULL : Size::TITLE);
		else
			MessengerOpenPacket(0).dispatch();
	}

	void UIMessenger::seat(int8_t slot)
	{
		auto messenger = UI::get().get_element<UIMessenger>();

		if (!messenger)
			messenger = UI::get().emplace<UIMessenger>();
		else if (!messenger->is_active())
			messenger->makeactive();

		if (!messenger)
			return;

		if (slot < 0)
		{
			messenger->join(0);
			messenger->add_note("- You have been unable to join the invited chat room.");
		}
		else
		{
			messenger->join(slot);

			if (static_cast<size_t>(slot) < SLOTS)
				messenger->add_note("- '" + Stage::get().get_player().get_name() + "' has entered.");
		}
	}

	void UIMessenger::set_member(uint8_t slot, const CharLook &look, const std::string &name, uint8_t channel, bool announce)
	{
		if (slot >= SLOTS)
			return;

		Member &member = members[slot];
		member.present = true;
		member.look = look;
		member.name.change_text(name);
		member.channel = channel;

		if (announce)
		{
			add_note("- '" + name + "' has entered.");
			unread = size == Size::TITLE;
			typing_gate.arrival();
		}
	}

	void UIMessenger::remove_member(uint8_t slot)
	{
		if (slot >= SLOTS || slot == own_slot)
			return;

		if (members[slot].present)
		{
			add_note("- '" + members[slot].name.get_text() + "' have left the room.");
			unread = size == Size::TITLE;
		}

		clear_member(slot);
	}

	void UIMessenger::update_look(uint8_t slot, const CharLook &look)
	{
		if (slot < SLOTS && members[slot].present)
			members[slot].look = look;
	}

	void UIMessenger::empty_seat(uint8_t slot)
	{
		if (slot < SLOTS)
			clear_member(slot);
	}

	void UIMessenger::hear(const std::string &line)
	{
		messenger_rules::Heard heard = messenger_rules::hear(line);

		for (size_t i = 0; i < SLOTS; i++)
		{
			Member &member = members[i];

			if (!member.present || member.name.get_text() != heard.name)
				continue;

			// Folded to its title the window doesn't follow who is typing.
			if (size != Size::TITLE)
			{
				if (heard.typing && !member.balloon_shown)
					member.balloon.reset();

				member.typing = heard.typing;
				member.balloon_shown = heard.typing;
				refresh_typing();
			}

			if (heard.chat)
			{
				size_t text = heard.name.size() + 3;
				std::string shown = line.substr(0, text) + CurseData::filter(line.substr(text));

				keep_said(shown);
				add_line(shown, SEAT_COLORS[i]);
				unread = size == Size::TITLE;
			}

			return;
		}
	}

	void UIMessenger::add_note(const std::string &note)
	{
		add_line(note, Color::Name::BLACK);
	}

	Button::State UIMessenger::button_pressed(uint16_t buttonid)
	{
		switch (buttonid)
		{
			case Buttons::MIN:
				set_size(size == Size::FULL ? Size::SHORT : Size::TITLE);

				return size == Size::TITLE ? Button::State::DISABLED : Button::State::NORMAL;
			case Buttons::MAX:
				set_size(size == Size::TITLE ? Size::SHORT : Size::FULL);

				return size == Size::FULL ? Button::State::DISABLED : Button::State::NORMAL;
			case Buttons::CLOSE:
				leave();
				break;
			case Buttons::ENTER:
				submit();

				return Button::State::DISABLED;
			case Buttons::CLAIM:
				UIClaimNotice::open("", said);
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
		set_member(slot, CharLook(), Stage::get().get_player().get_name(), Configuration::get().get_channelid());

		log.clear();
		log_height = 0;
		log_row = 0;
		said.clear();
		typing_gate = messenger_rules::TypingGate();
		history = messenger_rules::History();
		flood_guard = ChatFloodGuard();

		for (const std::string &line : HELP)
			add_note(line);

		unread = false;
		set_size(Size::FULL);
	}

	void UIMessenger::clear_member(uint8_t slot)
	{
		Member &member = members[slot];
		member.present = false;
		member.look = CharLook();
		member.name.change_text("");
		member.typing = false;
		member.balloon_shown = false;
		refresh_typing();
	}

	void UIMessenger::leave()
	{
		MessengerLeavePacket().dispatch();

		drop_focus();
		input.change_text("");
		clear_channel();

		deactivate();
	}

	void UIMessenger::drop_focus()
	{
		if (input.get_state() == Textfield::State::FOCUSED)
			input.set_state(Textfield::State::DISABLED);

		input.set_state(Textfield::State::NORMAL);
	}

	void UIMessenger::ask_leave()
	{
		drop_focus();

		UI::get().emplace<UIYesNo>(
				"Will you exit this chat room?",
				[](bool yes)
				{
					if (!yes)
						return;

					if (auto messenger = UI::get().get_element<UIMessenger>())
						messenger->leave();
				}
		);
	}

	void UIMessenger::submit()
	{
		std::string text = input.get_text();

		if (text.empty())
			return;

		input.change_text("");

		messenger_rules::Typed typed = messenger_rules::read(text);

		switch (typed.command)
		{
			case messenger_rules::Command::QUIT:
				leave();

				return;
			case messenger_rules::Command::INVITE:
				invite(typed.name);
				break;
			case messenger_rules::Command::CHAT:
				say(text);
				break;
			case messenger_rules::Command::NONE:
				break;
		}

		input.set_state(Textfield::State::FOCUSED);
		buttons[Buttons::ENTER]->set_state(Button::State::DISABLED);
	}

	void UIMessenger::say(const std::string &text)
	{
		typing_gate.sent();

		if (!UIChatBar::flood_allows(flood_guard, text))
			return;

		history.push(text);

		std::string line = Stage::get().get_player().get_name() + " : " + text;

		keep_said(line);
		add_line(line, SEAT_COLORS[own_slot]);
		MessengerChatPacket(line).dispatch();
	}

	void UIMessenger::invite(const std::string &name)
	{
		const std::string &own = Stage::get().get_player().get_name();

		if (messenger_rules::same_name(own, name))
			add_note("- Please enter the user's name.");
		else if (CurseData::in_name(name))
			add_note("- The character name is invalid.");
		else
			MessengerInvitePacket(name).dispatch();
	}

	void UIMessenger::add_line(const std::string &line, Color::Name color)
	{
		int16_t total = (log_height + LOG_ROW - 1) / LOG_ROW;
		bool at_bottom = log_row + log_rows() >= total;

		log.emplace_back(Text::Font::A11M, Text::Alignment::LEFT, color, line, LOG_WIDTH, false, LOG_ROW_ADJUST);
		log_height += rows_high(log.back());

		if (log.size() > LOG_LIMIT)
		{
			log_height -= rows_high(log.front());
			log.erase(log.begin());
		}

		if (at_bottom)
		{
			scroll_to_bottom();
		} else
		{
			int16_t rows = (log_height + LOG_ROW - 1) / LOG_ROW;

			// An old line going can leave fewer rows than we had scrolled past.
			log_row = std::min<int16_t>(log_row, std::max<int16_t>(0, rows - log_rows()));
			slider.setrows(log_row, log_rows(), rows);
		}
	}

	void UIMessenger::keep_said(const std::string &line)
	{
		said.push_back(line);

		if (said.size() > LOG_LIMIT)
			said.erase(said.begin());
	}

	void UIMessenger::scroll_to_bottom()
	{
		int16_t total = (log_height + LOG_ROW - 1) / LOG_ROW;

		log_row = std::max<int16_t>(0, total - log_rows());
		slider.setrows(log_row, log_rows(), total);
	}

	void UIMessenger::set_size(Size next)
	{
		size = next;

		// v83 builds a new input for the new size: what was typed goes.
		drop_focus();
		input.change_text("");

		bool title = size == Size::TITLE;
		const Layout &at = size == Size::FULL ? FULL_LAYOUT : SHORT_LAYOUT;
		const Texture &art = size == Size::FULL ? backgrnd : title ? title_backgrnd : short_backgrnd;

		dimension = art.get_dimensions();
		dragarea = Point<int16_t>(dimension.x(), title ? TITLE_DRAG_HEIGHT : DRAG_HEIGHT);

		int16_t button_y = title ? TITLE_BT_Y : BT_Y;

		buttons[Buttons::MIN]->set_position(Point<int16_t>(title ? TITLE_BT_MIN_X : BT_MIN_X, button_y));
		buttons[Buttons::MAX]->set_position(Point<int16_t>(title ? TITLE_BT_MAX_X : BT_MAX_X, button_y));
		buttons[Buttons::CLOSE]->set_position(Point<int16_t>(title ? TITLE_BT_CLOSE_X : BT_CLOSE_X, button_y));
		buttons[Buttons::MIN]->set_state(title ? Button::State::DISABLED : Button::State::NORMAL);
		buttons[Buttons::MAX]->set_state(size == Size::FULL ? Button::State::DISABLED : Button::State::NORMAL);

		buttons[Buttons::ENTER]->set_position(Point<int16_t>(BT_ENTER_X, at.enter_y));
		buttons[Buttons::ENTER]->set_active(!title);
		buttons[Buttons::ENTER]->set_state(Button::State::DISABLED);
		buttons[Buttons::CLAIM]->set_position(Point<int16_t>(BT_CLAIM_X, at.claim_y));
		buttons[Buttons::CLAIM]->set_active(!title);

		input.set_bounds(Rectangle<int16_t>(INPUT_LEFT, INPUT_RIGHT, at.input_top, at.input_top + INPUT_HEIGHT));
		slider.setvertical(Range<int16_t>(at.slider_top, at.slider_bottom));

		// The short window shows a balloon only on the next notice.
		for (Member &member : members)
			member.balloon_shown = size == Size::FULL && member.typing;

		if (!title)
			unread = false;

		clear_channel();
		scroll_to_bottom();
	}

	void UIMessenger::refresh_typing()
	{
		std::vector<std::string> names;

		for (const Member &member : members)
			if (member.present && member.typing)
				names.push_back(member.name.get_text());

		typing_row.change_text(messenger_rules::typing_row(names));
	}

	void UIMessenger::show_channel(Point<int16_t> cursorpos)
	{
		if (size == Size::FULL)
		{
			Point<int16_t> at = cursorpos - position;

			for (size_t i = 0; i < SLOTS; i++)
			{
				Rectangle<int16_t> avatar(SLOT_X[i] - AVATAR_HALF_WIDTH, SLOT_X[i] + AVATAR_HALF_WIDTH,
										  FEET_Y - AVATAR_HEIGHT, FEET_Y);

				if (members[i].present && avatar.contains(at))
				{
					UI::get().show_text(Tooltip::Parent::TEXT, "CH." + std::to_string(members[i].channel + 1));
					channel_tip = true;

					return;
				}
			}
		}

		clear_channel();
	}

	void UIMessenger::clear_channel()
	{
		if (channel_tip)
			UI::get().clear_tooltip(Tooltip::Parent::TEXT);

		channel_tip = false;
	}

	int16_t UIMessenger::log_rows() const
	{
		const Layout &at = size == Size::FULL ? FULL_LAYOUT : SHORT_LAYOUT;

		return (at.log_bottom - at.log_top) / LOG_ROW;
	}
}
