#include "../Gameplay/Combat/SameSpot.h"
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
		using namespace ms::same_spot;

		Limiter attacks(ATTACKS);
		// The first counts from the spot it moves to, then a hundred more.
		require(attacks.allows(100, 50), "the first moves the spot");

		for (int i = 0; i < ATTACKS; i++)
			require(attacks.allows(100 + i % 6, 50 + i % 150), "a hundred more from about there");

		require(!attacks.allows(105, 199), "then no more");
		require(!attacks.allows(95, -99), "on either side");
		require(attacks.allows(106, 50), "until a step aside");
		require(attacks.allows(106, 50), "which counts again");

		Limiter skills(SKILLS);
		require(skills.allows(300, 0), "a skill moves the spot too");

		for (int i = 0; i < SKILLS; i++)
			require(skills.allows(300, 0), "twenty more");

		require(!skills.allows(300, 0), "then none");
		require(skills.allows(300, 150), "until a jump down");

		require(!counts_shot(3121004) && !counts_shot(5221004) && !counts_shot(13111002), "held shots free");
		require(counts_shot(0) && counts_shot(3121003), "other shots counted");
	}
	catch (const std::exception& e)
	{
		std::cerr << e.what() << std::endl;
		return 1;
	}

	std::cout << "same_spot tests passed" << std::endl;
	return 0;
}
