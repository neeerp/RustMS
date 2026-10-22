#include "RpsGame.h"

namespace ms
{
	namespace
	{
		// The spin's step before the result, its growth after it (0xafde20), and the step it stops from.
		constexpr int32_t SPIN = 120;
		constexpr double SLOWING = 1.2;
		constexpr int32_t SLOWEST = 720;
		// The face and sound a second after the result, the next state two after that.
		constexpr int32_t FACE_DELAY = 1000;
		constexpr int32_t NEXT_DELAY = 3000;
		// The Timer ticks in the last ten seconds, each in the first 30 ms of its second.
		constexpr int32_t TICKING = 10000;
		constexpr int32_t TICK_WINDOW = 30;
		constexpr int8_t MOST_WINS = 10;

		constexpr int8_t SMILE = 2;
		constexpr int8_t TROUBLED = 3;
		constexpr int8_t CRY = 4;

		// StringPool 0xe5c to 0xe62.
		constexpr const char* PRESS_START = "Press 'Start' to start the game.";
		constexpr const char* SELECT = "Please select Rock, Paper, or Scissors in %d seconds.";
		constexpr const char* CHALLENGE =
			"To challenge the %d-game Winning Streak Challenge, press 'Continue'. A failed challenge will result in no reward.";
		constexpr const char* CONQUERED = "Congratulations on conquering the 10-game Winning Streak Challenge!";
		constexpr const char* TOO_LATE =
			"You have lost the game due to being over the time limit. Press 'Restart' to try again, which will cost you the participation fee of 1,000 mesos.";
		constexpr const char* CONSOLED =
			"Here's 500 mesos as a consolation prize. Press 'Restart' to try again, which will cost you the participation fee of 1,000 mesos.";
		constexpr const char* RESTART =
			"Press 'Restart' to try again, which will cost you the participation fee of 1,000 mesos.";

		std::string with_number(const char* text, int64_t number)
		{
			std::string line = text;
			size_t at = line.find("%d");

			return line.replace(at, 2, std::to_string(number));
		}

		// v83 keeps 0 for "unset", so a tick that is 0 becomes 1.
		int64_t tick(int64_t at)
		{
			return at ? at : 1;
		}
	}

	RpsGame::RpsGame() = default;

	void RpsGame::reset()
	{
		// 0x74034d: the hands and the choice stay as they were.
		interval = 0;
		streak = 0;
		deadline = 0;
		result_tick = 0;
		next_tick = 0;
		main_button = Main::START;
		main_visible = true;
		exit_visible = true;
	}

	void RpsGame::start_round(int64_t now)
	{
		// 0x7402e9: the spin goes on from where it stood.
		npc_hand = -1;
		interval = SPIN;
		last_spin = tick(now);
		deadline = tick(now + TIME_LIMIT);
		hands_visible = true;
	}

	void RpsGame::time_over(int64_t now)
	{
		interval = 0;
		npc_hand = -1;
		streak = -1;
		show_result(now);
	}

	void RpsGame::result(int8_t hand, int8_t s)
	{
		npc_hand = hand;
		consolation = s < 0 && streak == 0;
		streak = s;
	}

	void RpsGame::pressed()
	{
		main_visible = false;
		exit_visible = false;
	}

	void RpsGame::chose(int8_t c)
	{
		deadline = 0;
		chosen = c;
		hands_visible = false;
	}

	RpsGame::Events RpsGame::update(int64_t now)
	{
		Events events;

		if (interval && now - last_spin > interval)
		{
			spin_index = (spin_index + 1) % 3;
			last_spin = now;
			events.reel = true;

			if (npc_hand >= 0)
			{
				interval = static_cast<int32_t>(interval * SLOWING);

				if (interval >= SLOWEST && spin_index == npc_hand)
				{
					interval = 0;
					show_result(now);
				}
			}
		}

		if (deadline)
		{
			int64_t left = deadline - now;

			if (left < 0)
			{
				deadline = 0;
				hands_visible = false;
				events.time_up = true;
			}
			else if (left < TICKING && left % 1000 < TICK_WINDOW && left / 1000 != last_timer)
			{
				last_timer = left / 1000;
				events.timer = true;
			}
		}

		if (result_tick && now - result_tick >= 0)
		{
			result_tick = 0;

			banner_visible = true;

			if (streak < 0)
			{
				events.emotion = CRY;
				events.jingle = Jingle::LOOSE;
			}
			else if (draw())
			{
				events.emotion = TROUBLED;
				events.jingle = Jingle::DRAW;
			}
			else
			{
				events.emotion = SMILE;
				events.jingle = Jingle::WIN;
			}
		}

		if (next_tick && now - next_tick >= 0)
		{
			next_tick = 0;
			shown_banner = Banner::NONE;
			events.emotion = 0;

			if (draw())
			{
				// A draw is played again without a word to the server.
				npc_hand = -1;
				interval = SPIN;
				last_spin = tick(now);
				deadline = tick(now + TIME_LIMIT);
				hands_visible = true;
			}
			else
			{
				main_button = streak >= 1 && streak < MOST_WINS ? Main::CONTINUE : Main::RETRY;
				main_visible = true;
				exit_visible = true;
			}
		}

		return events;
	}

	RpsGame::Main RpsGame::main() const
	{
		return main_button;
	}

	bool RpsGame::main_shown() const
	{
		return main_visible;
	}

	bool RpsGame::exit_shown() const
	{
		return exit_visible;
	}

	bool RpsGame::hands_shown() const
	{
		return hands_visible;
	}

	RpsGame::Banner RpsGame::banner() const
	{
		return banner_visible ? shown_banner : Banner::NONE;
	}

	int8_t RpsGame::choice() const
	{
		return chosen;
	}

	int8_t RpsGame::spin() const
	{
		return spin_index;
	}

	bool RpsGame::spinning() const
	{
		return interval != 0;
	}

	bool RpsGame::update_status(int64_t now)
	{
		int32_t key = (main_visible ? 0x100 : 0) | static_cast<int32_t>(main_button);
		bool counting = !main_visible && main_button != Main::RETRY;

		if (key == status_key && !counting)
			return false;

		status_key = key;

		if (counting)
		{
			// Without a deadline the line stays as it was, the last second included.
			if (deadline)
				status_line = with_number(SELECT, (deadline - now + 999) / 1000);
		}
		else if (main_button == Main::START)
		{
			status_line = PRESS_START;
		}
		else if (main_button == Main::CONTINUE)
		{
			status_line = with_number(CHALLENGE, streak + 1);
		}
		else if (streak >= MOST_WINS)
		{
			status_line = CONQUERED;
		}
		else if (npc_hand < 0)
		{
			status_line = TOO_LATE;
		}
		else
		{
			// Hidden Restart keeps its line through the next game's rounds.
			status_line = consolation ? CONSOLED : RESTART;
		}

		return true;
	}

	const std::string& RpsGame::status() const
	{
		return status_line;
	}

	void RpsGame::show_result(int64_t now)
	{
		// 0x73eba9.
		if (streak < 0)
			shown_banner = npc_hand < 0 ? Banner::TIMEOVER : Banner::LOSE;
		else
			shown_banner = draw() ? Banner::DRAW : Banner::WIN;

		banner_visible = false;

		result_tick = tick(now + FACE_DELAY);
		next_tick = tick(now + NEXT_DELAY);
	}

	bool RpsGame::draw() const
	{
		return streak >= 0 && chosen == npc_hand;
	}
}
