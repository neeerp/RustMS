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

#include <algorithm>
#include <cctype>
#include <cstdlib>

#include "UIGroupPicker.h"
#include "UINotice.h"
#include "UIWhisperPrompt.h"
#include "UIStatusBar.h"

#include "../UI.h"

#include "../Components/MapleButton.h"

#include "../BuddyCommands.h"
#include "../GuildCommands.h"
#include "../PartyCommands.h"

#include "../../Configuration.h"
#include "../../Data/CurseData.h"
#include "../../Data/ItemData.h"
#include "../../Gameplay/Stage.h"
#include "../../Graphics/GraphicsGL.h"
#include "../../Net/Packets/MessagingPackets.h"
#include "../../Util/CharacterName.h"
#include "../../Util/ChatWrap.h"
#include "../../Util/NxFiles.h"

namespace ms
{
	namespace
	{
		enum class WhisperParseResult
		{
			NotWhisper,
			NoTarget,
			TargetOnly,
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
				return WhisperParseResult::NoTarget;

			size_t target_end = input.find(' ', target_start);
			target = input.substr(target_start, target_end - target_start);
			if (target_end == std::string::npos)
				return WhisperParseResult::TargetOnly;

			size_t message_start = input.find_first_not_of(' ', target_end);
			if (message_start == std::string::npos)
				return WhisperParseResult::TargetOnly;

			message = input.substr(message_start);
			return WhisperParseResult::ValidWhisper;
		}

		// v83 compares whisper targets without regard to case (_stricmp).
		bool same_name(const std::string& a, const std::string& b)
		{
			return a.size() == b.size() && std::equal(a.begin(), a.end(), b.begin(),
				[](unsigned char x, unsigned char y) { return std::tolower(x) == std::tolower(y); });
		}

		// Whether input is `name`, ignoring case, then one space and some
		// text: the command's argument (FUN_004a8e3d).
		bool parse_command_argument(const std::string& input, const std::string& name, std::string& argument)
		{
			const std::string COMMAND = name + " ";

			auto same = [](unsigned char a, unsigned char b) { return std::tolower(a) == std::tolower(b); };

			if (input.size() <= COMMAND.size() || !std::equal(COMMAND.begin(), COMMAND.end(), input.begin(), same))
				return false;

			argument = input.substr(COMMAND.size());
			return true;
		}

		// Whether input is /find, with the name it looks for.
		bool parse_find_command(const std::string& input, std::string& target)
		{
			return parse_command_argument(input, "/find", target);
		}

		// Whether input is exactly command, ignoring case, as v83 compares
		// commands (FUN_004a8def).
		bool is_command(const std::string& input, const std::string& command)
		{
			return input.size() == command.size() && std::equal(input.begin(), input.end(), command.begin(),
				[](unsigned char a, unsigned char b) { return std::tolower(a) == std::tolower(b); });
		}

		// A command that is "/" and the name of a face item (5160000-5160014)
		// makes its emote if the player has the item in the cash inventory, as
		// using it would; the first name that matches, compared as commands
		// are, decides (FUN_0052c40c, after /pet).
		void face_word(const std::string& input)
		{
			if (input.size() <= 1)
				return;

			std::string word = input.substr(1);

			for (int32_t index = 0; index < 15; index++)
			{
				int32_t item_id = 5160000 + index;
				const ItemData& item = ItemData::get(item_id);

				if (!item.is_valid() || !is_command(word, item.get_name()))
					continue;

				if (Stage::get().get_player().get_inventory().find_item(InventoryType::Id::CASH, item_id) != 0)
					Stage::get().send_key(KeyType::Id::FACE, 99 + index + 8, true);

				return;
			}
		}

		// v83's /? and /Help (StringPool 0x39a-0x3ac and 0x14c4).
		constexpr const char* HELP[] = {
			"----------------- HELP -----------------",
			"/partyinfo : Show the current party information.",
			"/makeparty : create a new party",
			"/leaveparty : If you are a member of the party, it'll let you leave..",
			"/partyinvite character-name : invite to our party (only for the party leader)",
			"/partyexpel character-name: Expel someone from the party (only for the party leader)",
			"/guildinfo : Give information about the guild",
			"/leaveguild : Quit the guild",
			"/guildinvite : Invite the user to the guild (only available to the master)",
			"/guildexpel : Expel the user from the guild (only available to the master)",
			"/find CharacterName : Give information about the chracter whether he or she is on-line or not.",
			"/gametrade charactername: Request to trade game items.",
			"/tradecash Charactername: Request to trade cash items.",
			"/buddy: Talk to your buddy.",
			"/party: Talk to party members.",
			"/guild: Talk to your guild members.",
			"/whisper charactername: Send a whisper (if no name is written, it'll be directed to your last contact).",
			"/all: Regular chat, not a whisper.",
			"* Open up the chat window and press tab, or close the chat window and",
			"press 1, 2, or 3 to change the group you'd like to talk to."
		};
		// A whisper or find unanswered this long no longer holds back the next.
		constexpr auto WHISPER_WAIT = std::chrono::milliseconds(3000);

		// The original fills the whole log with black at half opacity.
		constexpr float LOG_ALPHA = 0.5f;
		// It puts GM chat on a white strip of opacity 0xB0, and megaphones on
		// strips of their own (FUN_008db070).
		constexpr Color GM_STRIP = Color(0xFFFFFFB0);
		constexpr Color MEGAPHONE_STRIP = Color(0xCAE7FFB0);
		constexpr Color SUPER_MEGAPHONE_STRIP = Color(0xFFBFDDCC);
		constexpr Color ITEM_MEGAPHONE_STRIP = Color(0xFFC600DD);
		constexpr Color AVATAR_MEGAPHONE_STRIP = Color(0x4D1AAD2C);
		constexpr Color GACHAPON_STRIP = Color(0x99CC33FF);

		// A megaphone's channel tag is drawn this far into its first row,
		// whose text starts after it (FUN_008dbb93).
		constexpr Point<int16_t> TAG_POS = Point<int16_t>(4, 2);
		constexpr Point<int16_t> TENS_POS = Point<int16_t>(15, 4);
		constexpr Point<int16_t> ONES_POS = Point<int16_t>(20, 4);
		constexpr int16_t TAG_WIDTH = 38;

		// The input field, from the status bar's top left
		constexpr Rectangle<int16_t> CHATFIELD = Rectangle<int16_t>(87, 530, 5, 25);

		// The chat target box (FUN_008dfb36), which opens its list upwards
		// with a 16 pixel row per target, framed in a pixel more
		// (FUN_004c5fb9).
		constexpr Rectangle<int16_t> COMBO = Rectangle<int16_t>(1, 81, 8, 28);
		constexpr int16_t COMBO_ROW = 16;
		constexpr int16_t COMBO_LIST_WIDTH = 90;
		// Where the box's text and each row's starts, by the top of the text,
		// and how far above that Arial 11 is drawn here.
		constexpr Point<int16_t> COMBO_TEXT = Point<int16_t>(6, 3);
		constexpr int16_t COMBO_ROW_TEXT_X = 7;
		constexpr int16_t COMBO_TEXT_RAISE = 6;
		constexpr Color COMBO_BORDER = Color(0x999999FF);
		constexpr Color COMBO_FILL = Color(0x315573FF);
		constexpr Color COMBO_LIT = Color(0x559AB9FF);

		// The original places a line by the top of its text; text here is
		// placed this far above that.
		constexpr int16_t TEXT_RAISE = 7;

		// The original wraps a line at 550 pixels less the scroll bar's
		// width, and keeps the latest 64 rows (FUN_008db070).
		constexpr int16_t WRAP_WIDTH = 550;
		constexpr size_t MAX_ROWS = 64;
		// Wide enough that measuring a line never wraps it
		constexpr uint16_t MEASURE_WIDTH = 16'000;
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
		held_part = ScrollPart::NONE;
		edge_dragging = false;
		whisper_waiting = false;

		// As in the original, a saved height out of bounds is forgotten.
		int16_t saved_height = static_cast<int16_t>(Setting<ChatHeight>::get().load());

		if (saved_height < MIN_LOG_HEIGHT || saved_height > MAX_LOG_HEIGHT)
			saved_height = LOG_HEIGHT;

		set_log_height(saved_height);

		nl::node megaphone = NxFiles::ui_classic["UIWindow.img"]["Megaphone"];

		for (size_t i = 0; i < 2; i++)
			channel_tags[i] = megaphone["ch" + std::to_string(i)];

		for (size_t i = 0; i < 10; i++)
			channel_digits[i] = megaphone["Num"][std::to_string(i)];

		nl::node base = NxFiles::ui_classic["StatusBar.img"]["base"];
		nl::node basic = NxFiles::ui_classic["Basic.img"];
		nl::node scrollbar = basic["VScr5"];

		backgrnd = base["backgrnd"];
		backgrnd2 = base["backgrnd2"];

		nl::node combo = basic["ComboBox2"];
		combo_box[0] = combo["normal"]["0"];
		combo_box[1] = combo["mouseOver"]["0"];
		combo_box[2] = combo["pressed"]["0"];
		combo_hovered = false;
		combo_open = false;

		for (size_t i = 0; i < chat_target::COUNT; i++)
			combo_items[i] = Text(Text::Font::A11M, Text::Alignment::LEFT, Color::Name::WHITE,
								  chat_target::item_text(static_cast<Target>(i)));

		chattarget_label = Text(Text::Font::A11M, Text::Alignment::LEFT, Color::Name::WHITE, "");
		chat_target = Target::ALL;
		update_face();

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
					// The open target list takes Enter for the row lit.
					if (combo_open)
					{
						if (combo_row)
							set_target(static_cast<Target>(*combo_row));

						close_combo();
						return;
					}

					if (msg.size() > 0)
					{
						size_t last = msg.find_last_not_of(' ');

						if (last != std::string::npos)
						{
							msg.erase(last + 1);
							std::string target;
							std::string message;
							WhisperParseResult whisper_result = parse_whisper_command(msg, target, message);

							// Whisper picked by Tab, with no name to go to; the line
							// still reaches the pets (FUN_008d536c).
							if (msg[0] != '/' && chat_target == Target::WHISPER && whisper_name.empty())
								send_chatline("There is no one to whisper to.", LineType::SKILL);

							// HC's own: v83 has no command to whisper one line.
							if (whisper_result == WhisperParseResult::ValidWhisper
								&& same_name(target, Stage::get().get_player().get_stats().get_name()))
							{
								send_chatline("You can't whisper to yourself.", LineType::SKILL);
							}
							else if (whisper_result == WhisperParseResult::ValidWhisper)
							{
								send_whisper(target, message);
							}
							else if (whisper_result != WhisperParseResult::NotWhisper)
							{
								whisper_command(target);
							}
							else if (is_command(msg, "/?") || is_command(msg, "/Help"))
							{
								for (const char* line : HELP)
									send_chatline(line, LineType::YELLOW);
							}
							// Like the chat target keys, these only switch the target.
							else if (is_command(msg, "/party"))
							{
								set_target(Target::PARTY);
							}
							else if (is_command(msg, "/buddy"))
							{
								set_target(Target::BUDDY);
							}
							else if (is_command(msg, "/guild"))
							{
								set_target(Target::GUILD);
							}
							else if (is_command(msg, "/alliance"))
							{
								set_target(Target::ALLIANCE);
							}
							else if (is_command(msg, "/couple"))
							{
								set_target(Target::SPOUSE);
							}
							else if (is_command(msg, "/all"))
							{
								set_target(Target::ALL);
							}
							else if (parse_find_command(msg, target))
							{
								send_whisper(target, "");
							}
							else if (PartyCommands::run(msg))
							{
							}
							else if (GuildCommands::run(msg))
							{
							}
							else if (parse_command_argument(msg, "/pet", message))
							{
								// StringPool 0x110, for text the curse filter won't take.
								if (!CurseData::valid(message))
								{
									UI::get().emplace<UIOk>("Cursing is not allowed.", [](bool) {});
								}
								else
								{
									std::string filtered = CurseData::filter(message);
									Stage::get().get_player().pets_say(filtered, filtered != message);
								}
							}
							// Commands the client doesn't know aren't chat, whatever
							// the target; v83 sends them only for GMs.
							else if (msg[0] == '/')
							{
								face_word(msg);
							}
							else if (!flood_guard_allows(msg))
							{
								// Refused: nothing is sent or shown.
							}
							else if (!whisper_name.empty())
							{
								// Whatever the target, a name means a whisper, which
								// the pets don't hear.
								send_whisper(whisper_name, msg);
							}
							else
							{
								// The player's pets hear the line, whoever it is
								// for, and it is still said (FUN_008d54b3).
								Stage::get().get_player().command_pets(msg);
								send_to_target(msg);
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
					if (combo_open)
					{
						combo_row = combo_row && *combo_row > 0 ? *combo_row - 1 : 0;
					}
					else if (lastpos > 0)
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
					if (combo_open)
					{
						combo_row = combo_row ? std::min(*combo_row + 1, chat_target::COUNT - 1) : 0;
					}
					else if (lastentered.size() > 0 && lastpos < lastentered.size() - 1)
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
					if (combo_open)
						close_combo();
					else
						toggle_chatfield(false);
				}
		);

		chatfield.set_key_callback(
				KeyAction::Id::TAB,
				[&]()
				{
					tab_target();
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

		if (combo_shown())
		{
			combo_box[combo_open ? 2 : combo_hovered ? 1 : 0].draw(position + COMBO.get_left_top());
			chattarget_label.draw(position + COMBO.get_left_top() + COMBO_TEXT - Point<int16_t>(0, COMBO_TEXT_RAISE));
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
			int16_t row = rowpos - i;

			if (row < 0 || row > rowmax)
				break;

			int16_t top = logbottom - CHATROWHEIGHT * (i + 1);

			const ChatRow& shown = rowtexts[row];
			const Color& strip = shown.strip;

			if (!strip.invisible())
				graphics.drawrectangle(position.x(), position.y() + top, LOG_WIDTH - sbwidth, CHATROWHEIGHT, strip.r(),
									   strip.g(), strip.b(), strip.a());

			int16_t indent = 0;

			if (shown.channel >= 0)
			{
				// The channel counts from one, always in two digits.
				Point<int16_t> origin = position + Point<int16_t>(0, top);
				int16_t number = shown.channel + 1;

				channel_tags[shown.ear ? 0 : 1].draw(origin + TAG_POS);
				channel_digits[number / 10 % 10].draw(origin + TENS_POS);
				channel_digits[number % 10].draw(origin + ONES_POS);
				indent = TAG_WIDTH;
			}

			shown.text.draw(position + Point<int16_t>(3 + indent, top - TEXT_RAISE));
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

		if (combo_open)
		{
			Rectangle<int16_t> list = combo_list_bounds();
			Point<int16_t> origin = position + list.get_left_top();

			graphics.drawrectangle(origin.x(), origin.y(), list.width(), list.height(), COMBO_BORDER.r(),
								   COMBO_BORDER.g(), COMBO_BORDER.b(), COMBO_BORDER.a());
			graphics.drawrectangle(origin.x() + 1, origin.y() + 1, list.width() - 2, list.height() - 2,
								   COMBO_FILL.r(), COMBO_FILL.g(), COMBO_FILL.b(), COMBO_FILL.a());

			for (size_t i = 0; i < chat_target::COUNT; i++)
			{
				int16_t top = static_cast<int16_t>(COMBO_ROW * i + 1);

				if (combo_row == i)
					graphics.drawrectangle(origin.x() + 1, origin.y() + top, list.width() - 2, COMBO_ROW - 1,
										   COMBO_LIT.r(), COMBO_LIT.g(), COMBO_LIT.b(), COMBO_LIT.a());

				combo_items[i].draw(origin + Point<int16_t>(COMBO_ROW_TEXT_X, top - COMBO_TEXT_RAISE));
			}
		}
	}

	void UIChatBar::update()
	{
		UIElement::update();

		chatfield.update(position);

		for (auto iter : message_cooldowns)
			iter.second -= Constants::TIMESTEP;

		cost_cooldown -= Constants::TIMESTEP;

		if (whisper_waiting && std::chrono::steady_clock::now() - whisper_sent > WHISPER_WAIT)
			whisper_waiting = false;

		if (held_part != ScrollPart::NONE)
		{
			auto now = std::chrono::steady_clock::now();

			if (now - held_last > std::chrono::milliseconds(50) && scroll_part_at(held_cursor) == held_part)
			{
				held_last = now;
				scroll_step(held_part);
			}
		}
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
		// The open target list has the mouse to itself, so that a click
		// anywhere closes it.
		if (combo_open)
			return true;

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
		bool released = !clicking && scrollbar_pressed;
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

		// Like the original's mouse capture: the repeat in update follows
		// the cursor until the button is released.
		if (clicking)
			held_cursor = cursorpos;
		else
			held_part = ScrollPart::NONE;

		if (edge_dragging)
		{
			if (clicking)
			{
				drag_log_edge(cursorpos.y() - position.y());

				return Cursor::State::VSCROLLIDLE;
			}

			edge_dragging = false;
		}

		combo_hovered = combo_shown() && on_combo(cursorpos);

		if (combo_open)
		{
			// The row under the cursor lights up and is picked when the
			// button comes up on it (CCtrlComboBox).
			if (std::optional<size_t> row = combo_row_at(cursorpos))
			{
				combo_row = row;

				if (released)
					set_target(static_cast<Target>(*row));

				return clicking ? Cursor::State::CLICKING : Cursor::State::CANCLICK;
			}

			if (!pressed)
				return Cursor::State::IDLE;

			close_combo();

			if (on_combo(cursorpos))
				return Cursor::State::CLICKING;
		}

		if (combo_hovered)
		{
			if (pressed)
			{
				combo_open = true;
				combo_row = static_cast<size_t>(chat_target);
			}

			return clicking ? Cursor::State::CLICKING : Cursor::State::CANCLICK;
		}

		if (on_log_edge(cursorpos) && scroll_part_at(cursorpos) == ScrollPart::NONE)
		{
			if (pressed)
			{
				edge_dragging = true;

				if (!edge_anchor)
					edge_anchor = cursorpos.y() - position.y();
			}

			return Cursor::State::VSCROLLIDLE;
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

		arrow_pressed[Arrow::PREV] = false;
		arrow_pressed[Arrow::NEXT] = false;

		ScrollPart part = scroll_part_at(cursorpos);

		if (part == ScrollPart::NONE)
			return UIElement::send_cursor(clicking, cursorpos);

		if (get_scrollrange() < 2)
			return Cursor::State::IDLE;

		if (part == ScrollPart::PREV)
			arrow_pressed[Arrow::PREV] = clicking;
		else if (part == ScrollPart::NEXT)
			arrow_pressed[Arrow::NEXT] = clicking;

		if (pressed)
		{
			if (part == ScrollPart::THUMB)
			{
				thumb_dragging = true;
				drag_y = cursorpos.y();
				drag_pos = get_scrollpos();

				return Cursor::State::VSCROLLIDLE;
			}

			held_part = part;
			held_last = std::chrono::steady_clock::now();
			scroll_step(part);
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
		held_part = ScrollPart::NONE;
		edge_dragging = false;
		combo_hovered = false;
	}

	UIElement::Type UIChatBar::get_type() const
	{
		return TYPE;
	}

	void UIChatBar::send_chatline(const std::string &line, LineType type)
	{
		add_line(line, type, -1, false);
	}

	std::vector<std::string> UIChatBar::claim_rows() const
	{
		std::vector<std::string> rows;

		for (const auto& row : rowtexts)
			if (row.claimable)
				rows.push_back(row.line);

		return rows;
	}

	void UIChatBar::send_megaphone(const std::string &line, LineType type, int8_t channel, bool ear)
	{
		add_line(line, type, channel, ear);
	}

	void UIChatBar::add_line(const std::string &line, LineType type, int8_t channel, bool ear)
	{
		Color::Name color;
		Color strip;

		switch (type)
		{
			case LineType::RED:
				color = Color::Name::DARKRED;
				break;
			case LineType::BLUE:
				color = Color::Name::MAYABLUE;
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
				color = Color::Name::PIZAZZ;
				break;
			case LineType::PINK:
				color = Color::Name::MONALISA;
				break;
			case LineType::SKILL:
				color = Color::Name::SUNDOWN;
				break;
			case LineType::WHISPER:
				color = Color::Name::LIGHTGREEN;
				break;
			case LineType::GM:
				color = Color::Name::BLACK;
				strip = GM_STRIP;
				break;
			case LineType::GUILD:
				color = Color::Name::MAUVE;
				break;
			case LineType::ALLIANCE:
				color = Color::Name::ALLIANCEGREEN;
				break;
			case LineType::SPOUSE:
				color = Color::Name::SPOUSEPINK;
				break;
			case LineType::LIGHT_YELLOW:
				color = Color::Name::LIGHTYELLOW;
				break;
			case LineType::MEGAPHONE:
				color = Color::Name::MEGAPHONE;
				strip = MEGAPHONE_STRIP;
				break;
			case LineType::SUPER_MEGAPHONE:
				color = Color::Name::SUPERMEGAPHONE;
				strip = SUPER_MEGAPHONE_STRIP;
				break;
			case LineType::ITEM_MEGAPHONE:
				color = Color::Name::ITEMMEGAPHONE;
				strip = ITEM_MEGAPHONE_STRIP;
				break;
			case LineType::MEGAPHONE_PREVIEW:
				color = Color::Name::SILVER;
				break;
			case LineType::AVATAR_MEGAPHONE:
				color = Color::Name::AVATARMEGAPHONE;
				strip = AVATAR_MEGAPHONE_STRIP;
				break;
			case LineType::GACHAPON:
				color = Color::Name::BLACK;
				strip = GACHAPON_STRIP;
				break;
			default:
				color = Color::Name::WHITE;
				break;
		}

		// The original indents the rows after the first, except for its
		// notice and error colours.
		bool indent = type == LineType::UNK0 || type == LineType::WHITE || type == LineType::PARTY ||
					  type == LineType::BUDDY || type == LineType::WHISPER || type == LineType::GM ||
					  type == LineType::GUILD || type == LineType::ALLIANCE || type == LineType::SPOUSE ||
					  type >= LineType::MEGAPHONE;
		int16_t width = WRAP_WIDTH - arrows[Arrow::PREV][0].width();
		// The first row makes room for a channel tag.
		int16_t room = channel >= 0 ? width - TAG_WIDTH : width;

		auto fit = [&](const std::string& text)
		{
			Text measured(Text::Font::A12M, Text::Alignment::LEFT, color, text, MEASURE_WIDTH);
			size_t count = 0;

			while (count < text.size() && measured.advance(count + 1) <= room)
				count++;

			room = width;

			return count;
		};

		bool claimable = type == LineType::UNK0 || type == LineType::WHITE || type == LineType::PARTY ||
						 type == LineType::BUDDY || type == LineType::GUILD || type == LineType::WHISPER || type == LineType::MEGAPHONE ||
						 type == LineType::SUPER_MEGAPHONE || type == LineType::ITEM_MEGAPHONE ||
						 type == LineType::MEGAPHONE_PREVIEW || type == LineType::GACHAPON;

		for (const auto& row : ChatWrap::wrap(line, indent, fit))
		{
			rowtexts.push_back({ Text(Text::Font::A12M, Text::Alignment::LEFT, color, row), strip, channel, ear, row,
								 claimable });
			channel = -1;
		}

		while (rowtexts.size() > MAX_ROWS)
			rowtexts.pop_front();

		rowmax = static_cast<int16_t>(rowtexts.size()) - 1;
		rowpos = rowmax;
	}

	void UIChatBar::display_message(Messages::Type line, UIChatBar::LineType type, const std::string &item)
	{
		if (Messages::is_cost(line))
		{
			bool quiet = cost_cooldown > 0;
			cost_cooldown = COST_MESSAGE_COOLDOWN;

			if (quiet)
				return;
		} else if (line != Messages::SKILL_COOLDOWN)
		{
			// v83 says a skill is still cooling down on every try, with no
			// throttle (FUN_00966f7a).
			if (message_cooldowns[line] > 0)
				return;

			message_cooldowns[line] = line == Messages::ATTACK_NOWEAPON ? COST_MESSAGE_COOLDOWN : MESSAGE_COOLDOWN;
		}

		std::string message = Messages::messages[line];
		size_t name = message.find("%s");

		if (name != std::string::npos)
			message.replace(name, 2, item);

		send_chatline(message, type);
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

		if (!combo_shown())
			close_combo();

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

		if (!combo_shown())
			close_combo();
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
		close_combo();

		if (new_target == Target::GROUP)
			return ask_group();

		if (new_target == Target::WHISPER)
			return ask_whisper_name();

		focus_field();
		whisper_name.clear();
		chat_target = new_target;
		update_face();
	}

	void UIChatBar::whisper_to(const std::string& name)
	{
		whisper_name = name;

		if (!name.empty())
			chat_target = Target::WHISPER;

		update_face();
		focus_field();
	}

	void UIChatBar::ask_whisper_name()
	{
		const std::deque<std::string>& names = whisper_history.all();

		UI::get().emplace<UIWhisperPrompt>(
			std::vector<std::string>(names.begin(), names.end()),
			[this](const std::string& typed, const std::vector<std::string>& kept)
			{
				// OK keeps the names left in the list, before the name typed
				// is checked (FUN_008d4dbf).
				whisper_history.replace(kept);

				size_t first = typed.find_first_not_of(' ');

				if (first == std::string::npos)
					return;

				std::string name = typed.substr(first, typed.find_last_not_of(' ') + 1 - first);

				if (!character_name_valid(name))
					return send_chatline("Incorrect ID.", LineType::SKILL);

				if (same_name(name, Stage::get().get_player().get_stats().get_name()))
					return send_chatline("You can't whisper to yourself.", LineType::SKILL);

				add_whisper_history(name);
				whisper_to(name);
			}
		);
	}

	void UIChatBar::add_whisper_history(const std::string& name)
	{
		whisper_history.add(name);
	}

	void UIChatBar::whisper_command(const std::string& name)
	{
		// Alone, it goes back to the last name, or to none.
		if (name.empty())
			return whisper_to(whisper_history.latest());

		if (same_name(name, Stage::get().get_player().get_stats().get_name()))
			return send_chatline("You can't whisper to yourself.", LineType::SKILL);

		if (!character_name_valid(name))
			return send_chatline("Incorrect ID.", LineType::SKILL);

		add_whisper_history(name);
		whisper_to(name);
	}

	void UIChatBar::tab_target()
	{
		close_combo();

		Target next = chat_target::next_tab(chat_target);

		// From alliance, Tab keeps a name if there is one, and otherwise
		// whispers no one rather than asking who.
		if (next == Target::WHISPER)
		{
			if (!whisper_name.empty())
				return whisper_to(whisper_name);

			chat_target = Target::WHISPER;
			update_face();
			return;
		}

		set_target(next);
	}

	void UIChatBar::ask_group()
	{
		std::vector<std::string> groups;

		for (const Buddy& buddy : Stage::get().get_player().get_buddies().get_buddies())
			groups.push_back(buddy.group);

		UI::get().emplace<UIGroupPicker>(
			chat_target::group_choices(groups),
			[this](const std::string& group)
			{
				// OK whispers to the group typed, even a blank one
				// (FUN_008d4d43).
				focus_field();
				whisper_name.clear();
				group_name = group;
				chat_target = Target::GROUP;
				update_face();
			}
		);
	}

	void UIChatBar::focus_field()
	{
		toggle_chatfield(true);
		chatfield.change_text("");
	}

	void UIChatBar::update_face()
	{
		chattarget_label.change_text(chat_target::face_text(chat_target, group_name, whisper_name));
	}

	Rectangle<int16_t> UIChatBar::combo_list_bounds() const
	{
		int16_t height = static_cast<int16_t>(COMBO_ROW * chat_target::COUNT + 1);
		int16_t left = COMBO.left() + 1;

		return Rectangle<int16_t>(left, left + COMBO_LIST_WIDTH, COMBO.top() - height, COMBO.top());
	}

	std::optional<size_t> UIChatBar::combo_row_at(Point<int16_t> cursorpos) const
	{
		Rectangle<int16_t> list = combo_list_bounds();
		Point<int16_t> pos = cursorpos - position - list.get_left_top();

		if (pos.x() < 0 || pos.x() >= list.width() || pos.y() < 0 || pos.y() >= list.height())
			return std::nullopt;

		return std::min<size_t>(pos.y() / COMBO_ROW, chat_target::COUNT - 1);
	}

	bool UIChatBar::on_combo(Point<int16_t> cursorpos) const
	{
		Point<int16_t> pos = cursorpos - position;

		return pos.x() >= COMBO.left() && pos.x() < COMBO.right() && pos.y() >= COMBO.top() && pos.y() < COMBO.bottom();
	}

	bool UIChatBar::combo_shown() const
	{
		return chatfieldopen || chatopen;
	}

	void UIChatBar::close_combo()
	{
		combo_open = false;
		combo_row.reset();
	}

	bool UIChatBar::flood_guard_allows(const std::string& line)
	{
		auto now = std::chrono::duration_cast<std::chrono::milliseconds>(
			std::chrono::steady_clock::now().time_since_epoch()).count();

		switch (flood_guard.check(line, now))
		{
			case ChatFloodGuard::Result::SEND:
				return true;
			case ChatFloodGuard::Result::REPEATED:
				UI::get().emplace<UIOk>("Repeating the same line over and over\\ncan negatively affect other users.", [](bool) {});
				break;
			case ChatFloodGuard::Result::TOO_FAST:
				UI::get().emplace<UIOk>("Too much chatting can disrupt\\nother players' ability to play the game.", [](bool) {});
				break;
			case ChatFloodGuard::Result::MUTED:
				break;
		}

		return false;
	}

	void UIChatBar::send_whisper(const std::string& name, const std::string& message)
	{
		if (name == Stage::get().get_player().get_stats().get_name())
			return;

		if (whisper_waiting)
			return send_chatline("Please try again.", LineType::SKILL);

		if (message.empty())
			FindPacket(name).dispatch();
		else
			WhisperPacket(name, message).dispatch();

		// The line "name<< message" waits for the server's answer.
		whisper_waiting = true;
		whisper_sent = std::chrono::steady_clock::now();
		whisper_message = message;

		// With whispers blocked, an answer would be turned away (FUN_0052f185).
		if (!message.empty() && !Setting<AllowWhisper>::get().load())
			send_chatline("Since the character has the whispers blocked at this point, you may not receive an answer.", LineType::LIGHT_YELLOW);
	}

	std::string UIChatBar::whisper_answered()
	{
		whisper_waiting = false;
		return whisper_message;
	}

	void UIChatBar::send_party_chat(const std::string& message)
	{
		Player& player = Stage::get().get_player();
		const Party& party = player.get_party();

		std::vector<int32_t> recipients;

		for (auto& member : party.get_members())
			if (member.id != player.get_oid() && member.online())
				recipients.push_back(member.id);

		// v83 sends party chat only to members logged on (FUN_00a3f975).
		if (recipients.empty())
		{
			send_chatline("Either the party doesn't exist or no member of your party is logged on.", LineType::SKILL);
			return;
		}

		MultiChatPacket(MultiChatPacket::Mode::PARTY, recipients, message).dispatch();

		// The server sends the line to the other members only.
		send_chatline(player.get_stats().get_name() + " : " + message, LineType::PARTY);
	}

	void UIChatBar::send_buddy_chat(const std::string& message, const std::string* group)
	{
		Player& player = Stage::get().get_player();
		const BuddyList& buddies = player.get_buddies();
		// Blocked buddies are left out (FUN_00a3fb25).
		FriendGroups::Blocks blocks = BuddyCommands::blocks();
		std::vector<int32_t> recipients = group ? buddies.online_ids(*group, blocks.ids()) : buddies.online_ids(blocks.ids());

		if (recipients.empty())
		{
			send_chatline("You have no buddy available for chat.", LineType::SKILL);
			return;
		}

		MultiChatPacket(MultiChatPacket::Mode::BUDDY, recipients, message).dispatch();

		// As with party chat, the server sends the line to the others only.
		send_chatline(player.get_stats().get_name() + " : " + message, LineType::BUDDY);
	}

	void UIChatBar::send_guild_chat(const std::string& message)
	{
		Player& player = Stage::get().get_player();
		std::vector<int32_t> recipients = player.get_guild().online_ids(player.get_oid());

		// FUN_00a39209
		if (recipients.empty())
		{
			send_chatline("You are not in a guild yet or none of your guild member is online.", LineType::SKILL);
			return;
		}

		MultiChatPacket(MultiChatPacket::Mode::GUILD, recipients, message).dispatch();
		send_chatline(player.get_stats().get_name() + " : " + message, LineType::GUILD);
	}

	void UIChatBar::send_spouse_chat(const std::string& message)
	{
		const Player& player = Stage::get().get_player();
		const std::optional<MarriageRecord>& record = player.get_marriage();

		// Without a couple's record, engaged or married, v83 says so and
		// sends nothing; the server echoes a line that went (FUN_008defff).
		if (!record)
		{
			send_chatline(marriage::NO_SPOUSE, LineType::SKILL);
			return;
		}

		SpouseChatPacket(record->partner(player.get_stats().get_female()), message).dispatch();
	}

	void UIChatBar::send_alliance_chat(const std::string& message)
	{
		Player& player = Stage::get().get_player();
		std::vector<int32_t> recipients = player.get_alliance().online_ids(player.get_guild(), player.get_oid());

		// FUN_008df155
		if (recipients.empty())
		{
			send_chatline("There are no guild alliance you joined OR no one online in your guild alliance.",
						  LineType::SKILL);
			return;
		}

		MultiChatPacket(MultiChatPacket::Mode::ALLIANCE, recipients, message).dispatch();
		send_chatline(player.get_stats().get_name() + " : " + message, LineType::ALLIANCE);
	}

	void UIChatBar::send_to_target(const std::string& message)
	{
		switch (chat_target)
		{
			case Target::BUDDY:
				send_buddy_chat(message);
				break;
			case Target::GROUP:
				send_buddy_chat(message, &group_name);
				break;
			case Target::PARTY:
				send_party_chat(message);
				break;
			case Target::GUILD:
				send_guild_chat(message);
				break;
			case Target::ALLIANCE:
				send_alliance_chat(message);
				break;
			case Target::SPOUSE:
				send_spouse_chat(message);
				break;
			case Target::WHISPER:
				// No one to whisper to, as the player has been told.
				break;
			case Target::ALL:
				// Not while a dialog is open; a shop or trade has its own chat
				// (FUN_0052c315).
				if (UI::get().dialog_open())
					send_chatline("You can't chat right now.", LineType::SKILL);
				else
					GeneralChatPacket(message, false).dispatch();

				break;
		}
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

	// The original (FUN_0073ecb5) moves the edge in whole rows once the
	// cursor is more than a row from where the last step left it.
	void UIChatBar::drag_log_edge(int16_t y)
	{
		int16_t anchor = edge_anchor.value_or(y);
		bool shrinking = y >= get_logtop() + EDGE_OFFSET;
		int16_t distance = shrinking ? y - anchor : anchor - y;

		if (distance <= CHATROWHEIGHT)
			return;

		if (shrinking ? log_height <= MIN_LOG_HEIGHT : log_height >= MAX_LOG_HEIGHT)
			return;

		set_log_height(log_height + (anchor - y) / CHATROWHEIGHT * CHATROWHEIGHT);
		edge_anchor = get_logtop() + EDGE_OFFSET;
		rowpos = rowmax;

		Setting<ChatHeight>::get().save(log_height);
	}

	bool UIChatBar::on_log_edge(Point<int16_t> cursorpos) const
	{
		Point<int16_t> pos = cursorpos - position;

		return chatopen && pos.x() < LOG_WIDTH && std::abs(pos.y() - (get_logtop() + EDGE_OFFSET)) < 5;
	}

	void UIChatBar::set_log_height(int16_t height)
	{
		height = std::clamp(height, MIN_LOG_HEIGHT, MAX_LOG_HEIGHT);

		// The original keeps a height that is a whole number of rows two
		// pixels taller.
		if (height % CHATROWHEIGHT == 0)
			height += 2;

		log_height = height;
	}

	UIChatBar::ScrollPart UIChatBar::scroll_part_at(Point<int16_t> cursorpos) const
	{
		Point<int16_t> rel = cursorpos - position - get_scrollbar_pos();
		int16_t width = arrows[Arrow::PREV][0].width();
		int16_t length = get_scrollbar_length();
		int16_t button = arrows[Arrow::PREV][0].height();

		if (rel.x() < 0 || rel.x() >= width || rel.y() < 0 || rel.y() >= length)
			return ScrollPart::NONE;

		if (rel.y() < button)
			return ScrollPart::PREV;

		if (rel.y() >= length - button)
			return ScrollPart::NEXT;

		// The original only counts an arrow's height of the thumb.
		int16_t thumbtop = get_thumb_offset(button).value_or(button);

		if (rel.y() < thumbtop)
			return ScrollPart::PAGE_UP;

		if (rel.y() > thumbtop + button)
			return ScrollPart::PAGE_DOWN;

		return ScrollPart::THUMB;
	}

	void UIChatBar::scroll_step(ScrollPart part)
	{
		// The track pages through the log, at least sixteen lines at a time.
		int16_t page = std::max<int16_t>(16, get_scrollrange() / 16);

		switch (part)
		{
			case ScrollPart::PREV:
				scroll(-1);
				break;
			case ScrollPart::NEXT:
				scroll(1);
				break;
			case ScrollPart::PAGE_UP:
				set_scrollpos(get_scrollpos() - page);
				break;
			case ScrollPart::PAGE_DOWN:
				set_scrollpos(get_scrollpos() + page);
				break;
			default:
				break;
		}
	}

	int16_t UIChatBar::get_logrows() const
	{
		return chatopen ? log_height / CHATROWHEIGHT : 1;
	}

	int16_t UIChatBar::get_logheight() const
	{
		return chatopen ? log_height : LINE_LOG_HEIGHT;
	}

	// The original lays the chat out in a window 22 pixels below the top of
	// the screen, which puts the status bar 507 below that window's top. Its
	// log sits five pixels above where its mode says.
	int16_t UIChatBar::get_logtop() const
	{
		if (chatopen)
			return -3 - log_height;

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
		return chatopen ? log_height - 2 : LINE_LOG_HEIGHT + 1;
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

		// The collapsed log's scroll bar is just its two arrows, and the
		// original hides the thumb of a log under four rows tall.
		if (range < 2 || travel < 0 || (chatopen && log_height < 4 * CHATROWHEIGHT))
			return std::nullopt;

		return button + travel * get_scrollpos() / (range - 1);
	}
}
