#include "../Gameplay/MapleMap/Pickup.h"
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
		using namespace ms;
		using namespace ms::pickup;

		Point<int16_t> feet(100, 200);

		require(in_reach(feet, {100, 200}), "a drop at the feet is in reach");
		require(in_reach(feet, {75, 200}), "25 px to the left is in reach");
		require(!in_reach(feet, {125, 200}), "25 px to the right is just out of reach");
		require(in_reach(feet, {124, 200}), "24 px to the right is in reach");
		require(in_reach(feet, {100, 150}), "50 px above the feet is in reach");
		require(!in_reach(feet, {100, 149}), "51 px above the feet is out of reach");
		require(in_reach(feet, {100, 209}), "9 px below the feet is in reach");
		require(!in_reach(feet, {100, 210}), "10 px below the feet is out of reach");

		require(owned_by_other(0, 7, 8, 0, 0), "another character's drop is theirs at first");
		require(!owned_by_other(0, 8, 8, 0, 0), "your own drop is yours");
		require(!owned_by_other(0, 7, 8, 0, OWNERSHIP), "after 15 s anyone can take it");
		require(owned_by_other(1, 3, 8, 4, 14999), "another party's drop is theirs for 15 s");
		require(!owned_by_other(1, 4, 8, 4, 0), "your party's drop is yours");
		require(!owned_by_other(2, 7, 8, 0, 0), "a free-for-all drop is anyone's");
		require(!owned_by_other(3, 7, 8, 0, 0), "an explosive drop is anyone's");

		require(pet_owned_by_other(0, 7, 8, 0, 60000, false), "a pet never takes another's drop");
		require(!pet_owned_by_other(0, 8, 8, 0, 0, false), "a pet takes its owner's drop at once");
		require(!pet_owned_by_other(1, 4, 8, 4, 0, false), "and its owner's party's");
		require(!pet_owned_by_other(2, 7, 8, 0, 0, false), "and a free-for-all drop");
		require(pet_owned_by_other(0, 7, 8, 0, OWNERSHIP, true), "Magic Scales wait out the 15 s");
		require(!pet_owned_by_other(0, 7, 8, 0, OWNERSHIP + 1, true), "then take another's drop");

		std::cout << "Pickup passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
