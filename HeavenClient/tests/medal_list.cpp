#include "../Character/MedalList.h"
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
		using namespace ms::medal_list;

		std::vector<Medal> medals = {
			{ 29005, 1, 30, OBTAINABLE, 0 },
			{ 29004, 1, 15, CHALLENGING, 0 },
			{ 29006, 1, 15, OBTAINABLE, 0 },
			{ 29014, 1, 70, OBTAINED, 200 },
			{ 29900, 0, 10, OBTAINED, 100 },
			{ 29000, 2, 0, CHALLENGING, 0 }
		};

		auto general = rows(medals, 2, 0);
		require(general.size() == 5, "General: two headers and three titles, not the one obtained");
		require(general[0].quest < 0 && general[0].group == CHALLENGED && general[0].count == 1,
			"the titles being challenged come first, under their header");
		require(general[1].quest == 29004, "then that title");
		require(general[2].quest < 0 && general[2].group == AVAILABLE && general[2].count == 2,
			"then the obtainable ones' header");
		require(general[3].quest == 29006 && general[4].quest == 29005, "by level");

		auto folded = rows(medals, 2, 1 << AVAILABLE);
		require(folded.size() == 3 && folded[2].quest < 0 && folded[2].count == 2,
			"a folded header keeps its count but not its titles");

		auto job = rows(medals, 1, 0);
		require(job.empty(), "a tab whose titles are all obtained shows no headers");

		auto basic = rows(medals, BASIC, 0);
		require(basic.size() == 2 && basic[0].quest == 29014 && basic[1].quest == 29900,
			"Basic lists every title obtained, the latest first, without headers");

		require(rows(medals, 3, 0).size() == 2, "Challenge has its one title being challenged");
		require(rows(medals, 4, 0).empty(), "Event has none");
	}
	catch (const std::exception& e)
	{
		std::cerr << e.what() << std::endl;
		return 1;
	}

	std::cout << "medal_list tests passed" << std::endl;
	return 0;
}
