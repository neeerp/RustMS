#include "../IO/Components/SkillRecommendation.h"

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
		// Etc/RecommendSkill.img/100, in the file's (string) order.
		std::vector<std::pair<int32_t, int32_t>> warrior = {
			{ 0, 1001004 }, { 1, 1000000 }, { 16, 1001004 }, { 35, 1001005 }, { 54, 1001005 }, { 6, 1000001 }
		};

		require(ms::recommended_skill(warrior, 0) == 1001004, "no SP spent gives the first entry");
		require(ms::recommended_skill(warrior, 1) == 1000000, "1 SP reaches the second entry");
		require(ms::recommended_skill(warrior, 5) == 1000000, "the entry holds until the next one");
		require(ms::recommended_skill(warrior, 6) == 1000001, "entries are sorted by SP, not file order");
		require(ms::recommended_skill(warrior, 15) == 1000001, "15 SP is still before the 16 entry");
		require(ms::recommended_skill(warrior, 16) == 1001004, "16 SP reaches the 16 entry");
		require(ms::recommended_skill(warrior, 54) == 1001005, "the last entry applies at its SP");

		// Past the last entry, or a job without entries: nothing.
		require(ms::recommended_skill(warrior, 55) == 0, "nothing past the last entry");
		require(ms::recommended_skill({}, 0) == 0, "nothing for a job without entries");

		// Below the first entry: nothing.
		require(ms::recommended_skill({ { 3, 2001002 } }, 2) == 0, "nothing below the first entry");

		std::cout << "Skill recommendation passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
