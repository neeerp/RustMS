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

		std::cout << "skill macros passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
