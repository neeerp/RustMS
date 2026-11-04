#include "../Gameplay/Combat/AttackDash.h"

#include <iostream>
#include <stdexcept>
#include <vector>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

using namespace ms::attack_dash;

// Where each tick of `skill` goes until the steps run out, 0 for a wait.
static std::vector<int32_t> play(int32_t skill, bool left, int32_t area, bool mob_moves)
{
	Course course;
	course.start(skill, ms::attack_dash::course(skill, left, area, mob_moves));

	std::vector<int32_t> moves;

	while (true)
	{
		std::optional<Move> move = course.tick();

		if (!course.running())
			return moves;

		moves.push_back(move ? move->x : 0);
	}
}

static int32_t total(const std::vector<int32_t> &moves)
{
	int32_t sum = 0;

	for (int32_t move : moves)
		sum += move;

	return sum;
}

int main()
{
	try
	{
		require(course(1001005, false, -100, true).empty(), "other attacks don't move the player");

		std::vector<int32_t> rush{ 31, 31, 31, 24, 19, 15, 12, 9, 7 };
		require(play(RUSH_HERO, false, -250, true) == rush, "Rush holds 90 ms, then fades by a fifth");

		std::vector<int32_t> back{ -31, -31, -31, -24, -19, -15, -12, -9, -7 };
		require(play(RUSH_DK, true, -250, true) == back, "and goes the way the player faces");

		std::vector<int32_t> charge = play(FINAL_CHARGE, false, -250, true);
		require(charge.size() == 11 && charge[0] == 0 && charge[1] == 0 && charge[2] == 31,
				"Final Charge is a Rush 60 ms late");

		std::vector<int32_t> corkscrew = play(CORKSCREW_BLOW, false, 0, true);
		require(corkscrew[0] == 109 && corkscrew[4] == 9 && total(corkscrew) == 130, "Corkscrew leaps, then creeps");
		require(corkscrew.size() == 60, "and keeps the player for 1.8 s");

		std::vector<int32_t> backspin = play(BACKSPIN_BLOW, false, 0, true);
		require(backspin[15] == 0 && backspin[16] == -120 && total(backspin) == -129, "Backspin goes back, late");
		require(total(play(BACKSPIN_BLOW, true, 0, true)) == 129, "from either side");

		require(total(play(BARRAGE, false, 0, true)) == 14, "Barrage follows a mob that moves");
		require(total(play(BARRAGE, false, 0, false)) == 0, "and not one that can't");
		require(total(play(DOUBLE_SWING, true, 0, true)) == -15, "the Aran's swings follow too");
		require(total(play(OVER_SWING_1, false, 0, false)) == 0, "the same way");
		require(play(DOUBLE_SWING, false, 0, true).size() == 9, "Double Swing waits 240 ms");
		require(play(TRIPLE_SWING, false, 0, true).size() == 6, "Triple Swing 150 ms");

		require(marks_path(FINAL_CHARGE) && !marks_path(RUSH_HERO), "only the Aran marks the path");

		require(lock(RUSH_PALADIN) == 1770 && lock(CORKSCREW_BLOW_TB) == 1450 && lock(BACKSPIN_BLOW) == 1960,
				"the three locks");
		require(lock(BARRAGE) == 0 && lock(FINAL_CHARGE) == 0, "and no others");

		// A step reaches 30 px at least, except fading or going left past 30.
		Course reach;
		reach.start(RUSH_HERO, { { Kind::STEADY, 10, 30 }, { Kind::STEADY, 40, 30 }, { Kind::STEADY, -40, 30 },
								 { Kind::FADING, 20, 30 }, { Kind::FADING, -20, 30 } });
		require(reach.tick()->reach == 30, "short steps reach 30");
		require(reach.tick()->reach == 40, "long ones their length");
		require(reach.tick()->reach == 30, "long ones to the left only 30");
		require(reach.tick()->reach == 16, "a fading one its length");
		require(reach.tick()->reach == 16, "either way");

		Course stopped;
		stopped.start(BACKSPIN_BLOW, course(BACKSPIN_BLOW, false, 0, true));
		require(stopped.running(), "steps keep the player from harm");
		stopped.stop();
		require(!stopped.running() && !stopped.tick(), "until the floor runs out");

		std::cout << "attack_dash tests passed\n";
	}
	catch (const std::exception &e)
	{
		std::cerr << e.what() << "\n";
		return 1;
	}

	return 0;
}
