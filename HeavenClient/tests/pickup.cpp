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

		Point<int16_t> owner(0, 0);
		Point<int16_t> pet(-40, 0);

		require(in_sweep_reach(owner, pet, {124, 0}, false), "a drop 124 px from the owner is swept");
		require(!in_sweep_reach(owner, pet, {125, 0}, false), "125 px is too far");
		require(in_sweep_reach(owner, pet, {-124, 0}, false), "to either side");
		require(!in_sweep_reach(owner, pet, {-125, 0}, false), "but no further");
		require(in_sweep_reach(owner, pet, {274, 0}, true), "a Binocular reaches 274 px");
		require(!in_sweep_reach(owner, pet, {275, 0}, true), "and not 275");
		require(in_sweep_reach(owner, pet, {0, -49}, false), "49 px above the pet's feet");
		require(!in_sweep_reach(owner, pet, {0, -50}, false), "not 50");
		require(in_sweep_reach(owner, pet, {0, 9}, false), "9 px below them");
		require(!in_sweep_reach(owner, pet, {0, 10}, false), "not 10");
		require(!in_sweep_reach(owner, {-40, 100}, {0, 0}, false), "the height counts from the pet, not the owner");

		require(sweep_direction(-41, -40) == -1, "a drop to the left");
		require(sweep_direction(-40, -40) == -1, "one right under the pet counts as left");
		require(sweep_direction(-39, -40) == 1, "a drop to the right");

		std::array<int16_t, 3> petx = { 0, 50, 100 };

		require(sweeper_for(petx, { true, true, true }, 60) == 1, "the nearest pet walks to a drop");
		require(sweeper_for(petx, { true, false, true }, 60) == 2, "of those that can");
		require(sweeper_for(petx, { true, false, true }, 50) == 0, "the first on a tie");
		require(sweeper_for(petx, { false, false, false }, 50) == -1, "none if none can");

		require(!sweep_ready(100), "a pet waits while its owner moves");
		require(sweep_ready(101), "and goes once they have stood still over 100 ms");

		require(within_leash(pet, {259, 0}), "an owner 299 px right of the pet keeps it");
		require(!within_leash(pet, {260, 0}), "300 px right calls it back");
		require(within_leash(pet, {-340, 0}), "300 px left keeps it");
		require(!within_leash(pet, {-341, 0}), "301 px left calls it back");
		require(within_leash(pet, {-40, -200}), "200 px above keeps it");
		require(!within_leash(pet, {-40, 200}), "200 px below calls it back");

		std::cout << "Pickup passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
