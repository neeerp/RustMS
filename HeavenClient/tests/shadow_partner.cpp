#include "../Character/ShadowPartnerRules.h"
#include <iostream>
#include <stdexcept>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

int main()
{
	using namespace ms::shadow_partner;

	try
	{
		require(skill_for(411) == 4111002 && skill_for(412) == 4111002, "a Hermit's clone is Shadow Partner's");
		require(skill_for(1411) == 14111000, "a Night Walker's is their own");

		require(offset(true, false) == Offset{ -30, 0 }, "facing right, the clone stands to the left");
		require(offset(false, false) == Offset{ 30, 0 }, "facing left, it stands to the right");
		require(offset(true, true) == Offset{ 0, 50 } && offset(false, true) == Offset{ 0, 50 },
			"on a ladder or rope it hangs below");

		require(shown(true, false, false), "the buff shows the clone");
		require(!shown(false, false, false) && !shown(true, true, false) && !shown(true, false, true),
			"without the buff, dead or riding it does not show");
		require(dies(false, false) && dies(true, true), "it dies with the buff or its user");
		require(!dies(true, false), "mounting just hides it");

		require(frame(0, 3) == 0 && frame(2, 3) == 2, "the clone steps with the avatar");
		require(frame(4, 3) == 2, "and holds its last frame");
		require(frame(1, 0) == 0, "an action without frames gives the first");

		require(slid(0, -30, 0) == 0 && slid(0, -30, 100) == -15, "it slides over 200 ms");
		require(slid(30, -30, 200) == -30 && slid(30, -30, 500) == -30, "and stays there");

		std::cout << "Shadow Partner passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
