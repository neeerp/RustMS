#pragma once

#include "../UIElement.h"

#include "../Components/Slider.h"
#include "../Components/Textfield.h"

#include "../../Character/Look/CharLook.h"
#include "../../Graphics/Text.h"
#include "../../Graphics/Texture.h"
#include "../../Net/Handlers/MiniGame.h"
#include "../../Net/Handlers/MiniRoom.h"

#include <array>
#include <functional>
#include <vector>

namespace ms
{
	// What an Omok room and a match cards room share (v83 CMiniGameDlg,
	// 0x65dc70): the board on the left, the two players, their records and
	// the chat on the right. The server opens it when we open a game or sit
	// down at one; Exit is the only way out.
	class UIMiniGame : public UIElement
	{
	public:
		static constexpr Type TYPE = UIElement::Type::MINIGAME;
		static constexpr bool FOCUSED = false;
		static constexpr bool TOGGLED = false;

		// Where the two games put the same things.
		struct Layout
		{
			Point<int16_t> title;
			// Points, then wins, losses and ties.
			std::array<int16_t, 4> record_y;
			std::array<int16_t, 2> points_x;
			std::array<int16_t, 2> tally_x;
			Point<int16_t> empty_record;
			Point<int16_t> turn_text;
			Point<int16_t> time_text;
		};

		void draw(float inter) const override;

		void update() override;

		void remove_cursor() override;

		Cursor::State send_cursor(bool clicked, Point<int16_t> cursorpos) override;

		void send_scroll(double yoffset) override;

		void send_key(int32_t keycode, bool pressed, bool escape) override;

		UIElement::Type get_type() const override;

		uint8_t get_own_slot() const;

		// mini_room::OMOK or MATCH_CARDS.
		int8_t get_game_type() const;

		// Someone sat down, with their record (0x6e3bcc).
		void visit(const mini_room::Member &member, const mini_room::GameRecord &record);

		void change_look(uint8_t slot, const LookEntry &look);

		// A line the player in `slot` said, "name : text".
		void add_chat(uint8_t slot, const std::string &line);

		// The game's notice `id` about `name` (mode 6 sub 7).
		void game_message(uint8_t id, const std::string &name);

		// The other player left (0x6e3d06).
		void seat_left(uint8_t slot);

		// We left the room, for `reason`.
		void leave(int8_t reason);

		void tie_asked();

		void tie_denied();

		// The visitor is ready, or no longer.
		void set_ready(bool ready);

		// The game starts; the player in `waits` moves second. Match cards
		// come with the `deck`.
		virtual void start(uint8_t waits, std::vector<int32_t> deck) = 0;

		void result(bool tie, uint8_t winner, const std::array<mini_room::GameRecord, 2> &records);

		// The player in `slot` moves now: the last one's time ran out.
		virtual void turn(uint8_t slot) = 0;

		// Omok's own.
		virtual void undo_asked() {}

		// The answer to taking back a stone: refused, or the last `count`
		// went and the player in `slot` moves.
		virtual void undo_result(bool undone, uint8_t count, uint8_t slot) {}

		virtual void stone(const mini_game::Stone &stone) {}

		virtual void rejected(uint8_t code) {}

		// Match cards' own: a turn's first card, and its second, after
		// `first`, with how it went.
		virtual void flipped_first(uint8_t index) {}

		virtual void flipped_second(uint8_t index, uint8_t first, uint8_t code) {}

		// Closes the window without telling the server, which ends the game
		// itself when we leave the field.
		void close();

	protected:
		UIMiniGame(const mini_room::Enter &enter, nl::node backgrnd, const Layout &layout);

		Button::State button_pressed(uint16_t buttonid) override;

		virtual mini_game::Game &game() = 0;

		virtual const mini_game::Game &game() const = 0;

		// The board, over the background.
		virtual void draw_board(float inter) const = 0;

		// What sits on the record panels: each one's stone or pairs.
		virtual void draw_counters(float inter) const = 0;

		// The game's clock and animations; true when my time ran out and
		// the server is to hear of it.
		virtual bool tick() = 0;

		virtual int32_t seconds_left() const = 0;

		// A click on the board, released at `at`, window-local.
		virtual void click_board(Point<int16_t> at) = 0;

		// The mouse over the board at `at`, window-local, and whether it
		// has just gone down there; or the mouse leaving the board.
		virtual void hover_board(Point<int16_t> at, bool went_down) {}

		virtual void leave_board() {}

		virtual Rectangle<int16_t> board() const = 0;

		// Asks `question`, and if the answer is yes while this room is
		// still open, does `yes`.
		void ask(const std::string &question, std::function<void()> yes) const;

		void add_line(const std::string &line, Color::Name color);

		void play(const std::string &sound) const;

		bool full() const;

		// Which controls show and work, as the room and the game change.
		virtual void refresh_buttons();

		enum Buttons : uint16_t
		{
			START,
			READY,
			TIE,
			GIVE_UP,
			UNDO,
			EXIT,
			BAN,
			CLAIM,
		};

	private:
		struct Member
		{
			bool present = false;
			CharLook look;
			Text name;
			mini_room::GameRecord record;
			std::array<Text, 4> numbers;
		};

		void seat(Member &member, const mini_room::Member &entry, const mini_room::GameRecord &record);

		void show_record(Member &member);

		void exit();

		void submit(const std::string &text);

		int8_t game_type;
		Layout layout;
		// Tells this room from one opened after it, for a question asked here.
		uint32_t serial;
		// Whether the mouse was down at the last cursor event: the board acts on a release.
		bool mouse_down = false;

		Texture backgrnd;
		Texture name_bar;
		Texture record_panel;
		Texture empty_panel;
		Texture mark;
		std::array<Texture, 2> ready_sign;
		Texture turn_frame;
		Texture banner_win;
		Texture banner_lose;
		Texture banner_draw;
		Texture badge_win;
		Texture badge_lose;

		std::array<Member, 2> members;
		bool tournament;
		Text title;
		Text turn_text;
		Text time_text;
		// How long the result stays up.
		int32_t result_ms;

		struct Line
		{
			Text text;
			int16_t rows;
		};

		std::vector<Line> log;
		int16_t log_rows;
		int16_t log_row;
		Slider slider;
		Textfield input;
	};
}
