#include "../Gameplay/Combat/PointBlank.h"
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
		using point_blank::range;

		Rectangle<int16_t> bow = range({ -30, 0, -81, 5 });
		require(bow.left() == -30 && bow.right() == 0, "a bow's swing T1 keeps its 30 px reach");
		require(bow.top() == -81 && bow.bottom() == 5, "the height stays as it is");

		Rectangle<int16_t> claw = range({ -74, -27, -28, -18 });
		require(claw.left() == -65 && claw.right() == -27, "a claw's stab O1 reaches 65 px, not 74");

		Rectangle<int16_t> thin = range({ -90, -70, -10, 0 });
		require(thin.left() == -71 && thin.right() == -70, "a range starting past 65 px keeps 1 px");

		using point_blank::always_shoots;
		require(!always_shoots(0) && !always_shoots(3001005), "Double Shot whacks a mob in front");
		require(always_shoots(3111004) && always_shoots(21120006), "Arrow Rain and Combo Tempest always shoot");

		using point_blank::mortal_blow;
		require(mortal_blow(311) == 3110001 && mortal_blow(312) == 3110001, "a Ranger and a Bowmaster blow");
		require(mortal_blow(321) == 3210001 && mortal_blow(322) == 3210001, "a Sniper and a Marksman blow");
		require(!mortal_blow(310) && !mortal_blow(300) && !mortal_blow(411), "a Hunter and others don't");

		std::cout << "Point blank passed\n";
		return 0;
	}
	catch (const std::exception& e)
	{
		std::cerr << e.what() << '\n';
		return 1;
	}
}
