#include "../Character/Diseases.h"
#include <iostream>
#include <stdexcept>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

int main()
{
	using namespace ms::disease;

	try
	{
		require(skill(126 | 1 << 16) == 126 && level(126 | 1 << 16) == 1, "a reason is the mob skill and its level");
		require(is_disease(ms::Buffstat::SEDUCE) && is_disease(ms::Buffstat::SLOW), "Seduce and Slow are diseases");
		require(!is_disease(ms::Buffstat::SPEED), "Haste isn't");

		State healthy;
		require(steer(-1, 1, healthy) == Input{ -1, 1, false }, "the arrows steer a healthy player");
		require(can_jump(healthy) && can_attack(healthy) && can_use_skill(2001002, healthy), "who can do anything");

		State confused;
		confused.confused = true;
		require(steer(-1, 1, confused) == Input{ 1, -1, false }, "confusion reverses both arrows");

		State stunned;
		stunned.stunned = true;
		stunned.confused = true;
		require(steer(-1, 0, stunned) == Input{}, "a stun stops all input, confused or not");
		require(!can_jump(stunned) && !can_attack(stunned), "and jumps and attacks");
		require(!can_use_skill(DISPEL, stunned), "even Dispel");

		State attracted;
		attracted.seduced = 6;
		require(steer(1, 0, attracted) == Input{ -1, 0, true }, "seduction overrides the arrows");
		require(seduced(1) == Input{ -1, 0, false } && seduced(2) == Input{ 1, 0, false }, "1 and 2 walk");
		require(seduced(3) == Input{ 0, 0, true } && seduced(4) == Input{ 0, 1, false }, "3 jumps, 4 lies down");
		require(seduced(7) == Input{ 1, 0, true } && seduced(5) == Input{}, "7 jumps right; 5 does nothing");
		require(!can_jump(attracted) && !can_attack(attracted), "the player can't jump or attack by themself");

		State weakened;
		weakened.weakened = true;
		require(!can_jump(weakened) && can_attack(weakened), "weakness stops only jumping");

		State sealed;
		sealed.sealed = true;
		require(can_attack(sealed), "a seal leaves attacks");
		require(!can_use_skill(2001002, sealed) && can_use_skill(DISPEL, sealed), "and Dispel, but no other skill");

		State potionless;
		potionless.stop_potion = true;
		require(!can_use_item(potionless) && !can_use_item(attracted) && can_use_item(State{}),
				"Stop Potion and Seduce refuse items");

		require(stop_motion_damage(30, 500) == 30 && stop_motion_damage(30, 31) == 30, "Stop Motion takes its n");
		require(stop_motion_damage(30, 30) == 29 && stop_motion_damage(30, 1) == 0, "but never the last HP");

		require(given(ms::Buffstat::PUPPET, 133 | 1 << 16) == ms::Buffstat::UNDEAD, "bit 46 from mob skill 133 is Undead");
		require(given(ms::Buffstat::PUPPET, 3111002) == ms::Buffstat::PUPPET && given(ms::Buffstat::POISON, 133 | 1 << 16) == ms::Buffstat::POISON,
				"else it is what it was");

		require(walk_speed(140, 80) == 80 && walk_speed(140, std::nullopt) == 140, "Slow sets the speed");

		require(notice(123) == "You've been stunned, making you unable to move.", "a stun says so");
		require(notice(133) == "You will become an Undead and the effects of your Recovery will diminish by 50%.",
				"Undead says so");
		require(notice(136) == "The fear of darkness has overtaken you! Your range of vision has been narrowed.",
				"and Fear");
		require(notice(126).empty() && notice(132).empty(), "Slow and Confuse say nothing");

		std::cout << "Diseases passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
