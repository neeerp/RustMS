#include "../Character/SkillMacros.h"
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

		SkillMacros macros;

		require(macros.size() == 0 && macros.skill(0, 0) == 0 && macros.name(0).empty() && macros.shouts(0),
			"empty");

		SkillMacro buffs{ "buffs", true, { 1001003, 1101006, 0 } };
		SkillMacro hit{ "hit", false, { 0, 0, 1001004 } };
		macros.replace({ buffs, hit });

		require(macros.size() == 2, "two");
		require(macros.skill(0, 1) == 1101006 && macros.skill(1, 2) == 1001004, "skills by place");
		require(macros.skill(0, 3) == 0 && macros.skill(2, 0) == 0, "out of range");
		require(macros.name(1) == "hit" && !macros.shouts(0) && macros.shouts(1), "names and shouts");

		macros.replace(std::vector<SkillMacro>(7, hit));
		require(macros.size() == SkillMacros::MAX_MACROS, "cut to five");

		macros.replace({});
		require(macros.size() == 0, "replaced by none");

		macros.set_skill(2, 1, 1001004);
		require(macros.size() == 3 && macros.skill(2, 1) == 1001004, "a skill grows the list");
		require(macros.name(0).empty() && macros.shouts(0) && macros.skill(0, 0) == 0, "grown empty and shouting");

		macros.set_skill(5, 0, 1001004);
		macros.set_skill(0, 3, 1001004);
		require(macros.size() == 3 && macros.skill(0, 0) == 0, "no sixth macro or fourth slot");

		macros.clear_skill(2, 1);
		macros.clear_skill(4, 0);
		require(macros.size() == 3 && macros.skill(2, 1) == 0, "cleared in range only");

		require(!macros.set_name(0, "", true), "unchanged");
		require(macros.set_name(0, "", false) && !macros.shouts(0), "silenced");
		require(macros.set_name(0, "twelve bytes", false) && macros.name(0) == "twelve bytes", "renamed");
		require(!macros.set_name(0, "thirteen byte", false) && macros.name(0) == "twelve bytes", "too long");
		require(!macros.set_name(5, "a", true), "no sixth");
		require(!macros.set_name(4, "", true) && macros.size() == 5, "grows even when unchanged");

		std::cout << "skill macros passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
