#include "../Gameplay/MapleMap/MobHurts.h"
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
		using Status = MobStatParser::Status;

		std::map<uint8_t, Status> statuses;
		statuses[MobStatParser::POISON] = Status{ MobStatParser::POISON, 25, 2101005, 80 };
		statuses[MobStatParser::SPEED] = Status{ MobStatParser::SPEED, -20, 2101003, 40 };

		MobHurts hurts;
		hurts.start(MobStatParser::POISON);
		require(hurts.update(statuses, 0, 992).empty(), "nothing before a second");
		require(hurts.update(statuses, 0, 8) == std::vector<int32_t>{ 25 }, "the poison's value after a second");
		require(hurts.update(statuses, 0, 992).empty(), "then a second more");
		require(hurts.update(statuses, 0, 8) == std::vector<int32_t>{ 25 }, "and again");

		// Venom comes after Poison, both a second after they were put on;
		// a mob with fixedDamage shows that.
		statuses[MobStatParser::VENOM] = Status{ MobStatParser::VENOM, 300, 4120005, 8 };
		hurts.start(MobStatParser::VENOM);
		hurts.start(MobStatParser::POISON);
		require(hurts.update(statuses, 1, 1000) == std::vector<int32_t>{ 1, 1 }, "fixedDamage for both");
		require(hurts.update(statuses, 0, 1000) == std::vector<int32_t>{ 25, 300 }, "Poison, then Venom");

		// Gone, or at 0, it shows nothing; put on again, it waits anew.
		statuses.erase(MobStatParser::POISON);
		statuses[MobStatParser::VENOM].value = 0;
		require(hurts.update(statuses, 0, 5000).empty(), "nothing once they're gone");
		statuses[MobStatParser::AMBUSH] = Status{ MobStatParser::AMBUSH, 77, 4121004, 20 };
		hurts.start(MobStatParser::AMBUSH);
		require(hurts.update(statuses, 0, 999).empty(), "Ambush waits a second too");
		require(hurts.update(statuses, 0, 1) == std::vector<int32_t>{ 77 }, "then shows its value");

		// A mob only some skills hurt shows MISS for the others' numbers.
		std::vector<int32_t> flashfist{ 5001001 };
		require(hurts.update(statuses, 0, 1000, flashfist) == std::vector<int32_t>{ 0 }, "Ambush can't hurt it");
		require(MobHurts::hurt_by(flashfist, 5001001) && !MobHurts::hurt_by(flashfist, 0),
				"only Flash Fist hurts it, not a basic attack");
		require(MobHurts::hurt_by({}, 0), "any attack hurts other mobs");
		require(!MobHurts::hurts(MobStatParser::SPEED), "Slow doesn't hurt");
	}
	catch (const std::exception& e)
	{
		std::cerr << e.what() << std::endl;
		return 1;
	}

	return 0;
}
