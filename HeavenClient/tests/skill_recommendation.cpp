#include "../IO/Components/SkillRecommendation.h"

#include <cassert>

int main()
{
	// Etc/RecommendSkill.img/100, in the file's (string) order.
	std::vector<std::pair<int32_t, int32_t>> warrior = {
		{ 0, 1001004 }, { 1, 1000000 }, { 16, 1001004 }, { 35, 1001005 }, { 54, 1001005 }, { 6, 1000001 }
	};

	assert(ms::recommended_skill(warrior, 0) == 1001004);
	assert(ms::recommended_skill(warrior, 1) == 1000000);
	assert(ms::recommended_skill(warrior, 5) == 1000000);
	assert(ms::recommended_skill(warrior, 6) == 1000001);
	assert(ms::recommended_skill(warrior, 15) == 1000001);
	assert(ms::recommended_skill(warrior, 16) == 1001004);
	assert(ms::recommended_skill(warrior, 54) == 1001005);

	// Past the last entry, or a job without entries: nothing.
	assert(ms::recommended_skill(warrior, 55) == 0);
	assert(ms::recommended_skill({}, 0) == 0);

	// Below the first entry: nothing.
	assert(ms::recommended_skill({ { 3, 2001002 } }, 2) == 0);

	return 0;
}
