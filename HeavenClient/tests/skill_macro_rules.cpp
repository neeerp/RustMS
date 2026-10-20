#include "../Gameplay/SkillMacroRules.h"
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
		using namespace ms::skill_macro;

		constexpr int32_t RUSH = 1121006;
		auto cooltime = [](int32_t skill) { return skill == RUSH; };

		SkillMacros macros;
		macros.replace({
			{ "buffs", false, { 1001003, 0, 1101006 } },
			{ "rush", false, { RUSH, 1001004, 0 } },
			{ "skipped", false, { 1000000, 1009000, 0 } },
		});

		Runner runner;
		runner.start(macros, 0, cooltime);
		require(runner.running(), "queued");
		require(!runner.due(29) && runner.due(1), "one try per 30 ms");

		std::optional<Step> step = runner.due(30);
		require(step && step->skill == 1001003 && step->macro == 0 && step->first, "first skill");
		runner.tried(true);

		step = runner.due(30);
		require(step && step->skill == 1101006 && !step->first, "empty slot passed over");

		runner.start(macros, 1, cooltime);
		require(runner.due(30)->skill == 1101006, "no second macro while one runs");

		for (int32_t waited = 30; waited <= GIVE_UP; waited += 30)
		{
			runner.tried(false);
			require(runner.running(), "still trying");
			runner.due(30);
		}

		runner.tried(false);
		require(!runner.running(), "given up after 4 s");

		runner.start(macros, 1, cooltime);
		step = runner.due(30);
		require(step && step->skill == 1001004 && !step->first, "a cooldown skill is left out but keeps the name");
		runner.tried(true);
		require(!runner.running() && !runner.due(30), "done");

		runner.start(macros, 2, cooltime);
		require(!runner.running(), "passive and digit-9 skills skipped");
		runner.start(macros, 3, cooltime);
		runner.start(macros, -1, cooltime);
		require(!runner.running(), "no macro there");

		runner.start(macros, 0, cooltime);
		runner.clear();
		require(!runner.running(), "cleared");

		require(may_hold(1001004, false), "Power Strike");
		require(!may_hold(1000000, false) && !may_hold(1009000, false), "passive and digit 9");
		require(!may_hold(2121001, false) && !may_hold(5101004, false), "charged");
		require(!may_hold(2101002, false) && !may_hold(4111006, false) && !may_hold(4211006, false), "moves");
		require(!may_hold(1121006, true), "a cooldown");

		std::cout << "skill macro rules passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
