#include "../Character/RpsGame.h"

#include <iostream>
#include <stdexcept>

using namespace ms;

namespace
{
	void require(bool condition, const char* message)
	{
		if (!condition) throw std::runtime_error(message);
	}

	using Banner = RpsGame::Banner;
	using Jingle = RpsGame::Jingle;
	using Main = RpsGame::Main;

	// Runs the game frame by frame from `now` to `until`, gathering what happened.
	struct Run
	{
		int reels = 0;
		int timers = 0;
		bool time_up = false;
		Jingle jingle = Jingle::NONE;
		int8_t emotion = -1;
	};

	Run run(RpsGame& game, int64_t& now, int64_t until, int64_t frame = 8)
	{
		Run seen;

		for (; now < until; now += frame)
		{
			RpsGame::Events events = game.update(now);
			game.update_status(now);
			seen.reels += events.reel;
			seen.timers += events.timer;
			seen.time_up |= events.time_up;

			if (events.jingle != Jingle::NONE)
				seen.jingle = events.jingle;

			if (events.emotion >= 0)
				seen.emotion = events.emotion;
		}

		return seen;
	}

	// The line under the avatars at `now`.
	std::string line(RpsGame& game, int64_t now)
	{
		game.update_status(now);

		return game.status();
	}

	// Started and chosen: the server's answer is next.
	RpsGame chosen(int64_t& now, int8_t choice)
	{
		RpsGame game;
		game.pressed();
		game.start_round(now);
		game.chose(choice);

		return game;
	}

	void opens_with_start()
	{
		RpsGame game;
		require(game.main() == Main::START && game.main_shown() && game.exit_shown(), "start and exit");
		require(!game.hands_shown() && game.choice() == RpsGame::ROCK, "rock drawn, no hands");
		require(line(game, 0) == "Press 'Start' to start the game.", "start text");
	}

	void a_round_counts_down_until_a_hand_is_chosen()
	{
		int64_t now = 1000;
		RpsGame game;
		game.pressed();
		game.start_round(now);
		require(game.hands_shown() && !game.main_shown() && !game.exit_shown(), "hands only");
		require(line(game, now) == "Please select Rock, Paper, or Scissors in 30 seconds.", "30 seconds");
		require(line(game, now + 1) == "Please select Rock, Paper, or Scissors in 30 seconds.", "rounded up");
		require(line(game, now + 1000) == "Please select Rock, Paper, or Scissors in 29 seconds.", "29 seconds");
		require(game.update_status(now + 1000), "picked every frame");

		// The spin never slows before the result.
		Run seen = run(game, now, now + 2000);
		require(seen.reels >= 15 && seen.reels <= 17, "every 120 ms");
		require(game.banner() == Banner::NONE, "no result yet");
	}

	void the_last_ten_seconds_tick_and_then_time_runs_out()
	{
		int64_t now = 1000;
		RpsGame game;
		game.pressed();
		game.start_round(now);
		Run seen = run(game, now, now + 31000);
		require(seen.timers == 10, "ten ticks");
		require(seen.time_up && !game.hands_shown(), "time up hides the hands");

		game.time_over(now);
		require(game.banner() == Banner::NONE, "the banner waits");
		seen = run(game, now, now + 1001);
		require(game.banner() == Banner::TIMEOVER && seen.jingle == Jingle::LOOSE && seen.emotion == 4,
				"time over banner, loose and cry");
		seen = run(game, now, now + 2100);
		require(seen.emotion == 0 && game.banner() == Banner::NONE, "then back");
		require(game.main() == Main::RETRY && game.main_shown(), "retry");
		require(line(game, now).rfind("You have lost the game due to being over the time limit.", 0) == 0,
				"time over text");
	}

	void a_win_slows_the_spin_to_the_admins_hand()
	{
		int64_t now = 1000;
		RpsGame game = chosen(now, RpsGame::PAPER);
		run(game, now, now + 500);
		game.result(RpsGame::ROCK, 1);

		int64_t from = now;
		int64_t stopped = 0;

		for (; now < from + 8000 && !stopped; now += 8)
		{
			game.update(now);

			if (!game.spinning())
				stopped = now;
		}

		require(stopped, "the spin stops");
		require(game.spin() == RpsGame::ROCK && game.banner() == Banner::NONE, "on rock, the banner unseen");
		require(stopped - from > 3000 && stopped - from < 5000, "after three to five seconds");

		Run seen = run(game, now, stopped + 1001);
		require(game.banner() == Banner::WIN, "won");
		require(seen.jingle == Jingle::WIN && seen.emotion == 2, "win and smile a second later");
		require(!game.main_shown(), "nothing to press yet");
		seen = run(game, now, stopped + 3001);
		require(seen.emotion == 0 && game.banner() == Banner::NONE, "banner gone");
		require(game.main() == Main::CONTINUE && game.main_shown() && game.exit_shown(), "continue or exit");
		require(line(game, now) ==
				"To challenge the 2-game Winning Streak Challenge, press 'Continue'. A failed challenge will result in no reward.",
				"next challenge");
	}

	void a_draw_plays_again_by_itself()
	{
		int64_t now = 1000;
		RpsGame game = chosen(now, RpsGame::SCISSORS);
		game.result(RpsGame::SCISSORS, 0);
		Run seen = run(game, now, now + 9000);
		require(seen.jingle == Jingle::DRAW && seen.emotion == 0, "draw, troubled, then back");
		require(game.hands_shown() && !game.main_shown() && !game.exit_shown(), "choose again");
		require(line(game, now).rfind("Please select", 0) == 0, "counting down again");
	}

	void only_a_loss_from_no_streak_is_consoled()
	{
		int64_t now = 1000;
		RpsGame game = chosen(now, RpsGame::ROCK);
		game.result(RpsGame::PAPER, -1);
		Run seen = run(game, now, now + 9000);
		require(game.banner() == Banner::NONE && seen.jingle == Jingle::LOOSE && seen.emotion == 0, "lost");
		require(game.main() == Main::RETRY, "retry");
		require(line(game, now).rfind("Here's 500 mesos as a consolation prize.", 0) == 0, "consoled");

		// A new game keeps the lost streak, so its loss isn't; Restart's line stays through its rounds.
		game.pressed();
		require(line(game, now).rfind("Here's 500", 0) == 0, "no countdown after Restart");
		game.start_round(now);
		require(line(game, now).rfind("Here's 500", 0) == 0, "nor in its rounds");
		game.chose(RpsGame::ROCK);
		game.result(RpsGame::PAPER, -1);
		run(game, now, now + 9000);
		require(line(game, now) ==
				"Press 'Restart' to try again, which will cost you the participation fee of 1,000 mesos.",
				"not consoled");

		// A refusal starts over.
		game.reset();
		require(game.main() == Main::START && game.main_shown() && game.exit_shown(), "start again");
		game.pressed();
		game.start_round(now);
		game.chose(RpsGame::ROCK);
		game.result(RpsGame::PAPER, -1);
		run(game, now, now + 9000);
		require(line(game, now).rfind("Here's 500", 0) == 0, "consoled again");
	}

	void a_chosen_hand_leaves_the_last_second()
	{
		int64_t now = 1000;
		RpsGame game;
		game.pressed();
		game.start_round(now);
		run(game, now, now + 4500);
		game.chose(RpsGame::PAPER);
		require(line(game, now + 2000) == "Please select Rock, Paper, or Scissors in 26 seconds.", "left as it was");
	}

	void ten_wins_only_offer_retry()
	{
		int64_t now = 1000;
		RpsGame game = chosen(now, RpsGame::PAPER);
		game.result(RpsGame::ROCK, 10);
		run(game, now, now + 9000);
		require(game.main() == Main::RETRY, "retry");
		require(line(game, now) == "Congratulations on conquering the 10-game Winning Streak Challenge!",
				"conquered");
	}
}

int main()
{
	opens_with_start();
	a_round_counts_down_until_a_hand_is_chosen();
	the_last_ten_seconds_tick_and_then_time_runs_out();
	a_win_slows_the_spin_to_the_admins_hand();
	a_draw_plays_again_by_itself();
	a_chosen_hand_leaves_the_last_second();
	only_a_loss_from_no_streak_is_consoled();
	ten_wins_only_offer_retry();

	std::cout << "rps_game tests passed\n";

	return 0;
}
