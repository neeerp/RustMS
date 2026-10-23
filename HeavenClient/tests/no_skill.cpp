#include "../Gameplay/MapleMap/NoSkill.h"

#include <iostream>
#include <stdexcept>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

int main()
{
	try
	{
		using namespace ms::no_skill;

		require(advancement(0) == 1 && advancement(1000) == 1 && advancement(2000) == 1, "the beginner lines are 1");
		require(advancement(100) == 1 && advancement(500) == 1 && advancement(2100) == 1, "a first job is 1");
		require(advancement(110) == 2 && advancement(111) == 3 && advancement(112) == 4, "second to fourth");
		require(advancement(2001) == 1 && advancement(2200) == 1, "Evan's beginner and first");
		require(advancement(2210) == 2 && advancement(2218) == 10, "Evan's later ones run to 10");
		require(advancement(119) == 0 && advancement(115) == 0, "past the fourth is none");
		require(advancement(900) == 1 && advancement(910) == 2, "GM jobs go by the same digits");

		// Balrog's Tomb (105100300) takes the third and fourth jobs away.
		Lists balrog = { { 3, 4 }, {} };

		require(forbids(balrog, 1111002) && forbids(balrog, 1121008), "a crusader's and a hero's skill");
		require(!forbids(balrog, 1101004) && !forbids(balrog, 1001003), "the first and second jobs' go on");
		require(!forbids(balrog, 1002), "a beginner's goes on");

		// The Sheep Ranch (910040100) takes them all.
		Lists ranch = { { 1, 2, 3, 4 }, {} };

		require(forbids(ranch, 1002) && forbids(ranch, 10001002) && forbids(ranch, 2001003), "every job's");
		require(!forbids(ranch, 1190000), "a job of no advancement isn't matched");

		// Zakum's Altar (280030000) lists skills.
		Lists altar = { {}, { 1013, 1014, 1015, 10001014, 10001015, 10001016 } };

		require(forbids(altar, 1013) && forbids(altar, 10001016), "the listed skills");
		require(!forbids(altar, 10001013) && !forbids(altar, 1121008), "no other");

		Lists none;

		require(!forbids(none, 1002) && !forbids(none, 1121008), "a map with no noSkill");

		// A class of 0 matches nothing, though a job past its fourth has none.
		Lists zero = { { 0 }, {} };

		require(!forbids(zero, 1191000), "a 0 class takes nothing");

		std::cout << "no skill tests passed\n";
	}
	catch (const std::exception &e)
	{
		std::cerr << e.what() << '\n';
		return 1;
	}

	return 0;
}
