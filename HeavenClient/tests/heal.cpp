#include "../Gameplay/Combat/Heal.h"

#include <cmath>
#include <iostream>
#include <stdexcept>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

bool same(ms::Rectangle<int16_t> a, ms::Rectangle<int16_t> b)
{
	return a.left() == b.left() && a.right() == b.right() && a.top() == b.top() && a.bottom() == b.bottom();
}

bool near(double a, double b)
{
	return std::fabs(a - b) < 1e-6;
}

int main()
{
	try
	{
		using namespace ms;
		using namespace ms::heal;

		require(same(area({ -300, 300, -200, 200 }), { -180, 180, -120, 120 }), "60% of the skill's area");
		require(same(area({ -251, 251, -151, 149 }), { -150, 150, -90, 89 }), "cut towards the middle");

		require(near(base(300, 500, 400, 100, 1, 0.0), 2145.0), "the least, alone");
		require(near(base(300, 500, 400, 100, 1, 1.0), 5655.0), "the most rolls INT * 0.8");
		require(near(base(300, 500, 400, 100, 4, 0.0), 907.5), "reaching more, each gets less");
		require(near(base(300, 500, 400, 100, 0, 0.0), 2145.0), "the user always counts");
	} catch (const std::exception &e)
	{
		std::cerr << e.what() << '\n';
		return 1;
	}

	return 0;
}
