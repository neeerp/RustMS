#include "UIMiniGame.h"

#include "UINotice.h"

#include "../UI.h"

#include "../Components/MapleButton.h"

#include "../../Audio/Audio.h"
#include "../../Constants.h"
#include "../../Data/CurseData.h"
#include "../../Gameplay/Stage.h"
#include "../../Graphics/Geometry.h"
#include "../../Net/Packets/MiniRoomPackets.h"
#include "../../Util/NxFiles.h"

#include <algorithm>
#include <nlnx/nx.hpp>

namespace ms
{
	namespace
	{
		// The window, centred on v83's 800 by 600 screen and not to be
		// moved (0x4edaef, b100 §1.8).
		constexpr Point<int16_t> POSITION = Point<int16_t>(33, 85);

		// The controls, the same in both games (0x6e4773, 0x64e7a6).
		constexpr Point<int16_t> BT_CLAIM = Point<int16_t>(705, 371);
		constexpr Point<int16_t> BT_START = Point<int16_t>(625, 243);
		constexpr Point<int16_t> BT_TIE = Point<int16_t>(458, 403);
		constexpr Point<int16_t> BT_GIVE_UP = Point<int16_t>(410, 403);
		constexpr Point<int16_t> BT_EXIT = Point<int16_t>(679, 403);
		constexpr Point<int16_t> BT_BAN = Point<int16_t>(551, 63);

		// What Draw puts where (0x6e5215, 0x64f358), by slot.
		constexpr int16_t NAME_BAR_X[2] = { 404, 490 };
		constexpr int16_t NAME_BAR_Y = 149;
		constexpr int16_t NAME_X[2] = { 444, 530 };
		constexpr int16_t NAME_Y = 152;
		constexpr int16_t NAME_WIDTH = 73;
		constexpr int16_t RECORD_X = 404;
		constexpr int16_t RECORD_Y = 167;
		constexpr int16_t SLOT_PITCH = 86;
		constexpr Point<int16_t> FEET[2] = { Point<int16_t>(438, 141), Point<int16_t>(534, 141) };
		constexpr Point<int16_t> MARK = Point<int16_t>(460, 63);
		constexpr Point<int16_t> READY_SIGN = Point<int16_t>(495, 63);
		constexpr int16_t TURN_FRAME_X[2] = { 402, 488 };
		constexpr int16_t TURN_FRAME_Y = 55;
		constexpr Point<int16_t> BANNER = Point<int16_t>(87, 151);
		constexpr int16_t BADGE_X[2] = { 496, 410 };
		constexpr int16_t BADGE_Y = 113;

		// The chat (0x65f381): four rows of 16 on a half black box.
		constexpr Point<int16_t> LOG = Point<int16_t>(407, 287);
		constexpr Point<int16_t> LOG_SIZE = Point<int16_t>(318, 76);
		constexpr int16_t LOG_ROW = 16;
		constexpr int16_t LOG_ROWS = 4;
		constexpr int16_t LOG_WIDTH = 304;
		constexpr size_t LOG_LIMIT = 1000;
		constexpr int16_t SLIDER_X = 713;
		const Range<int16_t> SLIDER_RANGE = Range<int16_t>(287, 363);

		// v83 makes the edit 210 wide about x 511 (b100 §4.1).
		const Rectangle<int16_t> INPUT = Rectangle<int16_t>(406, 616, 374, 389);
		constexpr size_t INPUT_LIMIT = 256;

		// v83 draws a line's cap top about 3 below where it is drawn, we 9.
		constexpr Point<int16_t> TEXT_Y = Point<int16_t>(0, -6);

		// The result stays up this long (inferred).
		constexpr int32_t RESULT_MS = 3000;

		std::string named(std::string text, const std::string &name)
		{
			text.replace(text.find("%s"), 2, name);

			return text;
		}

		uint32_t next_serial()
		{
			static uint32_t serial = 0;

			return ++serial;
		}
	}

	UIMiniGame::UIMiniGame(const mini_room::Enter &enter, nl::node background, const Layout &layout) :
			UIElement(POSITION, Point<int16_t>(0, 0)), game_type(enter.type), layout(layout), serial(next_serial()),
			tournament(enter.tournament), result_ms(0), log_rows(0), log_row(0)
	{
		nl::node Common = NxFiles::ui_classic["UIWindow.img"]["Minigame"]["Common"];

		backgrnd = background;
		name_bar = Common["barBagic"];
		record_panel = Common["info2"];
		empty_panel = Common["info3"];
		mark = Common["mark"];
		ready_sign[false] = Common["readyOff"];
		ready_sign[true] = Common["readyOn"];
		turn_frame = Common["turn"];
		banner_win = Common["win"];
		banner_lose = Common["lose"];
		banner_draw = Common["draw"];
		badge_win = Common["charWin"];
		badge_lose = Common["charLose"];

		buttons[Buttons::CLAIM] = std::make_unique<MapleButton>(NxFiles::ui_classic["Basic.img"]["BtClaim"], BT_CLAIM);
		buttons[Buttons::START] = std::make_unique<MapleButton>(Common["btStart"], BT_START);
		buttons[Buttons::READY] = std::make_unique<MapleButton>(Common["btReady"], BT_START);
		buttons[Buttons::TIE] = std::make_unique<MapleButton>(Common["btDraw"], BT_TIE);
		buttons[Buttons::GIVE_UP] = std::make_unique<MapleButton>(Common["btAbsten"], BT_GIVE_UP);
		buttons[Buttons::BAN] = std::make_unique<MapleButton>(Common["btBan"], BT_BAN);
		buttons[Buttons::EXIT] = std::make_unique<MapleButton>(Common["btExit"], BT_EXIT);

		for (Member &member : members)
		{
			member.name = Text(Text::Font::A12M, Text::Alignment::CENTER, Color::Name::BLACK);

			for (Text &number : member.numbers)
				number = Text(Text::Font::A12M, Text::Alignment::RIGHT, Color::Name::BLACK);
		}

		for (const mini_room::Member &member : enter.members)
			if (member.slot < members.size())
				seat(members[member.slot], member, enter.records[member.slot]);

		title = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, enter.title);
		turn_text = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK);
		time_text = Text(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK);

		slider = Slider(
				Slider::Type::CLASSIC, SLIDER_RANGE, SLIDER_X, LOG_ROWS, 0,
				[&](bool upwards)
				{
					int16_t shift = upwards ? -1 : 1;

					if (log_row + shift >= 0 && log_row + shift + LOG_ROWS <= log_rows)
						log_row += shift;
				}
		);

		input = Textfield(Text::Font::A12M, Text::Alignment::LEFT, Color::Name::BLACK, INPUT, INPUT_LIMIT);

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

		// The owner opens the room to the map as it shows (0x6e388e,
		// 0x64dbac).
		if (enter.own_slot == 0)
			GameOpenPacket().dispatch();

		play("Enter");

		dimension = backgrnd.get_dimensions();
	}

	void UIMiniGame::draw(float inter) const
	{
		backgrnd.draw(position);

		draw_board(inter);

		uint8_t own_slot = get_own_slot();
		const mini_game::Game &state = game();

		for (uint8_t slot = 0; slot < members.size(); slot++)
		{
			const Member &member = members[slot];

			name_bar.draw(position + Point<int16_t>(NAME_BAR_X[slot], NAME_BAR_Y));

			if (!member.present)
				continue;

			record_panel.draw(position + Point<int16_t>(RECORD_X + SLOT_PITCH * slot, RECORD_Y));

			// Our own avatar follows our equipment; the two face each other.
			const CharLook &look = slot == own_slot ? Stage::get().get_player().get_look() : member.look;
			look.draw(position + FEET[slot], slot == 0, Stance::Id::STAND1, Expression::Id::DEFAULT);

			member.name.draw(position + Point<int16_t>(NAME_X[slot], NAME_Y) + TEXT_Y);
			member.numbers[0].draw(position + Point<int16_t>(layout.points_x[slot], layout.record_y[0]) + TEXT_Y);

			for (size_t i = 1; i < member.numbers.size(); i++)
				member.numbers[i].draw(position + Point<int16_t>(layout.tally_x[slot], layout.record_y[i]) + TEXT_Y);
		}

		if (!members[1].present)
			empty_panel.draw(position + layout.empty_record);

		draw_counters(inter);

		mark.draw(position + MARK);

		if (full())
			ready_sign[state.ready].draw(position + READY_SIGN);

		title.draw(position + layout.title + TEXT_Y);

		if (state.playing())
		{
			turn_frame.draw(position + Point<int16_t>(TURN_FRAME_X[state.turn_slot()], TURN_FRAME_Y));
			turn_text.draw(position + layout.turn_text + TEXT_Y);
			time_text.draw(position + layout.time_text + TEXT_Y);
		}

		// Only while the game is over: a new one may start before it fades.
		if (result_ms > 0 && state.ended())
		{
			// Over the opponent, the badge says how they did (0x6e6ef9).
			if (full() && !state.tie)
				(state.won() ? badge_lose : badge_win).draw(position + Point<int16_t>(BADGE_X[own_slot], BADGE_Y));

			const Texture &banner = state.tie ? banner_draw : state.won() ? banner_win : banner_lose;
			banner.draw(position + BANNER);
		}

		ColorBox(LOG_SIZE.x(), LOG_SIZE.y(), Color::Name::BLACK, 0.5f).draw(position + LOG);

		Range<int16_t> clip = Range<int16_t>(position.y() + LOG.y(), position.y() + LOG.y() + LOG_SIZE.y());
		int16_t y = LOG.y() + 2 - log_row * LOG_ROW;

		for (const Line &line : log)
		{
			if (y + line.rows * LOG_ROW > LOG.y() && y < LOG.y() + LOG_ROWS * LOG_ROW)
				line.text.draw(DrawArgument(position + Point<int16_t>(LOG.x() + 2, y) + TEXT_Y), clip);

			y += line.rows * LOG_ROW;
		}

		slider.draw(position);
		input.draw(position + TEXT_Y);

		UIElement::draw_buttons(inter);
	}

	void UIMiniGame::update()
	{
		UIElement::update();

		input.update(position);

		if (result_ms > 0)
		{
			result_ms -= Constants::TIMESTEP;

			if (result_ms <= 0)
			{
				game().settle();
				refresh_buttons();
			}
		}

		// The time runs out on my turn: the server passes it on (0x6e4f68,
		// 0x64efcc).
		if (tick())
			GamePacket(GamePacket::SKIP).dispatch();

		if (game().playing())
		{
			const Member &mover = members[game().turn_slot()];

			turn_text.change_text("It's [ " + mover.name.get_text() + " ]'s turn.");
			time_text.change_text("Time left : " + std::to_string(seconds_left()) + " sec.");
		}
	}

	void UIMiniGame::remove_cursor()
	{
		UIElement::remove_cursor();

		slider.remove_cursor();
		mouse_down = false;
		leave_board();
	}

	Cursor::State UIMiniGame::send_cursor(bool clicked, Point<int16_t> cursorpos)
	{
		Point<int16_t> at = cursorpos - position;

		// v83 places a stone or turns a card when the mouse comes up over the
		// board, wherever it went down (0x6e4d1f, 0x64ece1).
		bool released = mouse_down && !clicked;
		bool went_down = clicked && !mouse_down;
		mouse_down = clicked;

		if (Cursor::State sstate = slider.send_cursor(at, clicked))
			return sstate;

		Cursor::State tstate = input.send_cursor(cursorpos, clicked);

		if (tstate != Cursor::State::IDLE)
			return tstate;

		if (board().contains(at))
		{
			hover_board(at, went_down);

			if (released)
				click_board(at);

			UIElement::remove_cursor();

			// A held button stays held while the mouse moves, so the turn
			// waits for the release.
			return clicked ? Cursor::State::CLICKING : Cursor::State::IDLE;
		}

		leave_board();

		return UIElement::send_cursor(clicked, cursorpos);
	}

	void UIMiniGame::send_scroll(double yoffset)
	{
		slider.send_scroll(yoffset);
	}

	void UIMiniGame::send_key(int32_t, bool, bool)
	{
		// Escape leaves no game: only Exit does (0x6e4e8b).
	}

	UIElement::Type UIMiniGame::get_type() const
	{
		return TYPE;
	}

	uint8_t UIMiniGame::get_own_slot() const
	{
		return game().own_slot;
	}

	int8_t UIMiniGame::get_game_type() const
	{
		return game_type;
	}

	void UIMiniGame::visit(const mini_room::Member &member, const mini_room::GameRecord &record)
	{
		if (member.slot >= members.size())
			return;

		seat(members[member.slot], member, record);

		game().ready = false;
		refresh_buttons();

		add_line(named("[%s] have entered.", member.name), Color::Name::YELLOW);
		play("Enter");
	}

	void UIMiniGame::change_look(uint8_t slot, const LookEntry &look)
	{
		if (slot < members.size() && slot != get_own_slot() && members[slot].present)
			members[slot].look = CharLook(look);
	}

	void UIMiniGame::add_chat(uint8_t slot, const std::string &line)
	{
		if (slot == get_own_slot())
			add_line(line, Color::Name::WHITE);
		else
			add_line(mini_room::filter_chat(line, CurseData::filter), Color::Name::WHITE);
	}

	void UIMiniGame::game_message(uint8_t id, const std::string &name)
	{
		Color::Name color = mini_game::message_about_player(id) ? Color::Name::YELLOW : Color::Name::BRIGHTRED;

		for (const std::string &line : mini_game::message_lines(id, name))
			add_line(line, color);
	}

	void UIMiniGame::seat_left(uint8_t slot)
	{
		if (slot >= members.size() || slot == get_own_slot() || !members[slot].present)
			return;

		add_line(named("[%s] have left.", members[slot].name.get_text()), Color::Name::YELLOW);

		members[slot].present = false;
		game().ready = false;
		refresh_buttons();
	}

	void UIMiniGame::leave(int8_t reason)
	{
		// The other player leaving ends a tournament's room (0x6e3d06).
		if (!mini_game::leave_closes(reason))
		{
			game_message(0x67, "");

			return;
		}

		std::string text = mini_game::leave_text(reason);

		close();

		if (!text.empty())
			UI::get().emplace<UIOk>(text, [](bool) {});
	}

	void UIMiniGame::tie_asked()
	{
		UI::get().emplace<UIYesNo>("Your opponent requests a tie.\\nWill you accept it?", [](bool yes)
			{
				GamePacket(GamePacket::TIE, yes).dispatch();
			});
	}

	void UIMiniGame::tie_denied()
	{
		game().tie_denied();

		add_line("Your opponent denied your request for a tie.", Color::Name::BRIGHTRED);
	}

	void UIMiniGame::set_ready(bool ready)
	{
		game().ready = ready;
		refresh_buttons();
	}

	void UIMiniGame::result(bool tie, uint8_t winner, const std::array<mini_room::GameRecord, 2> &records)
	{
		game().end(tie, winner);

		for (uint8_t slot = 0; slot < members.size(); slot++)
		{
			members[slot].record = records[slot];
			show_record(members[slot]);
		}

		if (tie)
		{
			add_line("It's a tie.", Color::Name::BRIGHTRED);
			play("Draw");
		}
		else if (game().won())
		{
			add_line("You win.", Color::Name::BRIGHTRED);
			play("Win");
		}
		else
		{
			add_line("You lost.", Color::Name::BRIGHTRED);
			play("Loose");
		}

		result_ms = RESULT_MS;
		refresh_buttons();
	}

	void UIMiniGame::close()
	{
		if (input.get_state() == Textfield::State::FOCUSED)
			input.set_state(Textfield::State::DISABLED);

		input.set_state(Textfield::State::NORMAL);
		input.change_text("");

		deactivate();
	}

	Button::State UIMiniGame::button_pressed(uint16_t buttonid)
	{
		mini_game::Game &state = game();

		switch (buttonid)
		{
			case Buttons::START:
				if (full() && state.ready && !state.playing())
					GamePacket(GamePacket::START).dispatch();

				break;
			case Buttons::READY:
				if (!state.playing())
				{
					GamePacket(state.ready ? GamePacket::UNREADY : GamePacket::READY).dispatch();
					play("Ready");
				}

				break;
			case Buttons::TIE:
				if (state.can_ask_tie())
					ask("Will you request a tie?", [this]()
						{
							if (game().can_ask_tie())
							{
								game().ask_tie();
								GamePacket(GamePacket::ASK_TIE).dispatch();
							}
						});

				break;
			case Buttons::GIVE_UP:
				if (state.can_give_up())
					ask("Are you sure you want to give up?", [this]()
						{
							if (game().can_give_up())
							{
								game().give_up();
								GamePacket(GamePacket::GIVE_UP).dispatch();
							}
						});

				break;
			case Buttons::BAN:
				if (full())
					ask("Will you expel the user?", []()
						{
							GamePacket(GamePacket::EXPEL).dispatch();
						});

				break;
			case Buttons::EXIT:
				exit();
				break;
			default:
				// Like the status bar's, the claim button does nothing.
				break;
		}

		return Button::State::NORMAL;
	}

	void UIMiniGame::ask(const std::string &question, std::function<void()> yes) const
	{
		UI::get().emplace<UIYesNo>(question, [yes, serial = serial](bool answer)
			{
				auto room = UI::get().get_element<UIMiniGame>();

				if (answer && room && room->is_active() && room->serial == serial)
					yes();
			});
	}

	void UIMiniGame::add_line(const std::string &line, Color::Name color)
	{
		bool at_bottom = log_row + LOG_ROWS >= log_rows;

		Text text = Text(Text::Font::A12M, Text::Alignment::LEFT, color, line, LOG_WIDTH, true, 2);
		auto rows = static_cast<int16_t>(std::max(1, (text.height() + LOG_ROW - 3) / LOG_ROW));

		log.push_back({ text, rows });
		log_rows += rows;

		if (log.size() > LOG_LIMIT)
		{
			log_rows -= log.front().rows;
			log_row = std::max<int16_t>(0, log_row - log.front().rows);
			log.erase(log.begin());
		}

		if (at_bottom)
			log_row = std::max<int16_t>(0, log_rows - LOG_ROWS);

		slider.setrows(log_row, LOG_ROWS, log_rows);
	}

	void UIMiniGame::play(const std::string &sound) const
	{
		Sound(nl::nx::sound["MiniGame.img"][sound]).play(100);
	}

	bool UIMiniGame::full() const
	{
		return members[0].present && members[1].present;
	}

	void UIMiniGame::refresh_buttons()
	{
		bool owner = get_own_slot() == 0;
		bool playing = game().playing();

		// The owner starts, the visitor says they are ready; neither
		// shows during a game (0x6e388e, 0x64f222).
		buttons[Buttons::START]->set_active(owner && !playing);
		buttons[Buttons::READY]->set_active(!owner && !playing);
		buttons[Buttons::START]->set_state(full() && game().ready ? Button::State::NORMAL : Button::State::DISABLED);

		// Only the owner expels, once someone is there to expel.
		buttons[Buttons::BAN]->set_active(owner);
		buttons[Buttons::BAN]->set_state(full() && !tournament ? Button::State::NORMAL : Button::State::DISABLED);
	}

	void UIMiniGame::seat(Member &member, const mini_room::Member &entry, const mini_room::GameRecord &record)
	{
		member.present = true;
		member.look = CharLook(entry.look);
		member.record = record;
		show_record(member);

		// Cut to fit the bar.
		std::string name = entry.name;
		member.name.change_text(name);

		while (!name.empty() && member.name.width() + 5 > NAME_WIDTH)
		{
			name.pop_back();
			member.name.change_text(name);
		}
	}

	void UIMiniGame::show_record(Member &member)
	{
		member.numbers[0].change_text(std::to_string(member.record.points));
		member.numbers[1].change_text(std::to_string(member.record.wins));
		member.numbers[2].change_text(std::to_string(member.record.losses));
		member.numbers[3].change_text(std::to_string(member.record.ties));
	}

	void UIMiniGame::exit()
	{
		// During a game, Exit asks to leave once it is over, or no longer
		// (0x6e8e29).
		if (game().playing())
		{
			if (game().leave_after)
				ask("Will you cancel the request\\nto leave after this game?", [this]()
					{
						if (game().leave_after)
						{
							game().toggle_leave_after();
							GamePacket(GamePacket::STAY_AFTER).dispatch();
						}
					});
			else
				ask("Will you call to leave after this game?", [this]()
					{
						if (!game().leave_after)
						{
							game().toggle_leave_after();
							GamePacket(GamePacket::LEAVE_AFTER).dispatch();
						}
					});

			return;
		}

		ask("Are you sure you want to leave?", [this]()
			{
				MiniRoomLeavePacket().dispatch();
				close();
			});
	}

	void UIMiniGame::submit(const std::string &text)
	{
		// The server adds our name and sends the line back.
		if (!text.empty())
			MiniRoomChatPacket(text).dispatch();
	}
}
