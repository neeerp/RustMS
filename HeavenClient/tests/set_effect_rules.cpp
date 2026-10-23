#include "../Character/SetEffectRules.h"
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
		using namespace ms::set_effect;

		require(parts_of(1002317) == std::vector<int16_t>{ 1 }, "a cap on the head");
		require(parts_of(1102096) == std::vector<int16_t>{ 9 } && parts_of(1092030) == std::vector<int16_t>{ 10 },
				"a cape and a shield");
		require(parts_of(1112203) == std::vector<int16_t>({ 12, 13, 15, 16 }), "a ring on any finger");
		require(parts_of(1302016) == std::vector<int16_t>{ 11 } && parts_of(1702001) == std::vector<int16_t>{ 11 },
				"a weapon and a cash weapon");
		require(parts_of(5000002).empty() && parts_of(30000).empty(), "nothing worn");

		// Set 10: a cap, an overall and either of two pairs of shoes.
		Set ten{ Kind::PLAIN, { { 1, { 1002317 } }, { 5, { 1052026 } }, { 7, { 1070002, 1071008 } } }, {} };
		Parts worn = {};
		worn[1] = 1002317;
		worn[5] = 1052026;
		worn[7] = 1071008;
		std::array<int32_t, 3> no_pets = {};

		require(matches(ten, worn, 0, no_pets), "one of each group");
		worn[7] = 1072000;
		require(!matches(ten, worn, 0, no_pets), "a group missing");
		worn[7] = 1070002;

		// The group's name isn't the part: the earring's group 3 looks in the earrings' part.
		Set earring{ Kind::PLAIN, { { 3, { 1032042 } } }, {} };
		Parts ears = {};
		ears[4] = 1032042;
		require(matches(earring, ears, 0, no_pets), "by the item's part");
		ears[4] = 0;
		ears[3] = 1032042;
		require(!matches(earring, ears, 0, no_pets), "not by the group's name");

		// Group 0 is the hair.
		Set hair{ Kind::PLAIN, { { 0, { 30030, 30040 } } }, {} };
		Parts bald = {};
		bald[0] = 30040;
		require(matches(hair, bald, 0, no_pets), "the hair");

		// The cash weapon stands in for the weapon under it.
		Set weapon{ Kind::CASH, { { 11, { 1302016, 1702001 } } }, {} };
		Parts armed = {};
		armed[11] = 1302016;
		require(matches(weapon, armed, 0, no_pets), "the weapon");
		require(!matches(weapon, armed, 1702999, no_pets), "covered by another cash weapon");
		armed[11] = 1302000;
		require(matches(weapon, armed, 1702001, no_pets), "the cash weapon");

		Set ring{ Kind::CASH, { { 12, { 1112203 } } }, {} };
		Parts finger = {};
		finger[16] = 1112203;
		require(matches(ring, finger, 0, no_pets), "the fourth ring");

		// Every pet out must be on the list, and all three must be out.
		Set pets{ Kind::MULTIPET, {}, { 5000002, 5000011, 5000012 } };
		require(matches(pets, {}, 0, { 5000002, 5000011, 5000011 }), "three listed pets");
		require(!matches(pets, {}, 0, { 5000002, 5000011, 0 }), "two out");
		require(!matches(pets, {}, 0, { 5000002, 5000011, 5000044 }), "one not listed");

		require(match({ ten, earring }, ears, 0, no_pets) == -1, "none");
		require(match({ earring, ten }, worn, 0, no_pets) == 1, "the one completed");

		// A cash set wins, then a plain one, then one of pets.
		Parts both = worn;
		both[11] = 1302016;
		std::array<int32_t, 3> three = { 5000002, 5000011, 5000012 };
		require(match({ pets, ten, weapon }, both, 0, three) == 2, "cash first");
		require(match({ pets, ten }, both, 0, three) == 1, "then plain");
		require(match({ pets, earring }, both, 0, three) == 0, "then pets");
		require(match({ ten, Set{ Kind::PLAIN, { { 1, { 1002317 } } }, {} } }, worn, 0, no_pets) == 0,
				"the first of a kind");
		require(match({ pets, pets }, both, 0, three) == 1, "a second pets' set takes the plain place");
		require(match({ pets, pets, ten }, both, 0, three) == 1, "before a plain one after it");

		// The background fades as the character moves, and returns after 1.5 s of standing.
		Background background;
		require(!step(background, true, 0) && background.shown, "shown at first");
		require(!step(background, false, 100) && !background.shown, "hidden on moving");
		require(!step(background, false, 3000), "still moving");
		require(!step(background, true, 4500) && !background.shown, "not yet back");
		require(step(background, true, 4508) && background.shown, "back");
		require(!step(background, true, 4600), "back only once");

		require(fade(1.0f, false, 250) == 0.5f && fade(0.1f, false, 250) == 0.0f, "fades out over 500 ms");
		require(fade(0.0f, true, 250) == 0.5f && fade(0.9f, true, 250) == 1.0f, "and in");

		// Three variants go one to each pet; two go to the first two, and the third has either again.
		auto first = [](size_t) { return size_t{ 0 }; };
		auto last = [](size_t left) { return left - 1; };
		require(roll(3, first) == std::array<int32_t, 3>{ 0, 2, 1 }, "three, each once");
		require(roll(3, last) == std::array<int32_t, 3>{ 2, 1, 0 }, "three, the other way");
		require(roll(2, first) == std::array<int32_t, 3>{ 0, 1, 0 }, "two, then again");
		require(roll(2, last) == std::array<int32_t, 3>{ 1, 0, 1 }, "two, the other way");
		require(roll(1, last) == std::array<int32_t, 3>{ 0, 0, 0 } && roll(0, last) == roll(1, last), "one");

		std::cout << "Set effect rules passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
