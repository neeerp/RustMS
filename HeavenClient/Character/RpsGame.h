#pragma once

#include <cstdint>
#include <string>

namespace ms
{
	// The Rock, Paper, Scissors window's game (v83 0x73d05b, its Update 0x73f140; report b038): what the window
	// shows and when, from RPS_GAME and the clock. The server decides every round; the client only spins the
	// admin's hand to where it says and reads the streak's sign. Pure state: times are ticks in ms, and each
	// update says what to play or send.
	class RpsGame
	{
	public:
		// The one button in the middle, swapped in place (0x73fea9).
		enum class Main : uint8_t
		{
			START,
			CONTINUE,
			RETRY
		};

		// The result shown over the hands (window+0x114..0x120).
		enum class Banner : uint8_t
		{
			NONE,
			DRAW,
			WIN,
			LOSE,
			TIMEOVER
		};

		// MiniGame.img's sound a second after the result.
		enum class Jingle : uint8_t
		{
			NONE,
			WIN,
			DRAW,
			LOOSE
		};

		struct Events
		{
			// The spin moved on: Reel.
			bool reel = false;
			// A second of the last ten went by: Timer.
			bool timer = false;
			// The 30 seconds ran out: send RPS_ACTION 2.
			bool time_up = false;
			Jingle jingle = Jingle::NONE;
			// The face to show: 2 smile, 3 troubled, 4 cry, 0 back to normal; -1 to leave it.
			int8_t emotion = -1;
		};

		static constexpr int8_t ROCK = 0;
		static constexpr int8_t PAPER = 1;
		static constexpr int8_t SCISSORS = 2;
		static constexpr int32_t TIME_LIMIT = 30000;

		RpsGame();

		// RPS_GAME 6, 7 and 14: Start again.
		void reset();
		// 9 and 12: a round, with the hands to choose from.
		void start_round(int64_t now);
		// 10: out of time.
		void time_over(int64_t now);
		// 11: the admin's hand, where the spin is to stop, and the streak, negative on a loss.
		void result(int8_t hand, int8_t streak);

		// The middle button or Exit was pressed: both hide until the server answers.
		void pressed();
		// A hand was chosen, which is sent.
		void chose(int8_t choice);

		Events update(int64_t now);

		Main main() const;
		bool main_shown() const;
		bool exit_shown() const;
		bool hands_shown() const;
		Banner banner() const;
		// The player's hand drawn (rock before the first choice).
		int8_t choice() const;
		// The admin's hand drawn: where the spin is.
		int8_t spin() const;
		// Whether the spin is going.
		bool spinning() const;

		// Picks the line under the avatars again if v83 would (0x73f60d): on each change of the middle button or
		// of its showing, and every frame while Start or Continue is hidden, which is when it counts down. True if
		// it did, which also sends a scrolling line back to its start.
		bool update_status(int64_t now);
		const std::string& status() const;

	private:
		void show_result(int64_t now);
		bool draw() const;

		Main main_button = Main::START;
		bool main_visible = true;
		bool exit_visible = true;
		bool hands_visible = false;
		Banner shown_banner = Banner::NONE;
		// The banner is put up unseen and shows with the face (0x73f09c, 0x73f39d).
		bool banner_visible = false;

		int8_t chosen = ROCK;
		int8_t npc_hand = -1;
		int8_t spin_index = 0;
		int64_t last_spin = 0;
		int32_t interval = 0;
		int64_t deadline = 0;
		int64_t last_timer = -1;
		int8_t streak = 0;
		bool consolation = false;
		int64_t result_tick = 0;
		int64_t next_tick = 0;

		int32_t status_key = -1;
		std::string status_line;
	};
}
