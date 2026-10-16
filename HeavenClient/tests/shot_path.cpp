#include "../Gameplay/Combat/ShotPath.h"
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
		using namespace shot_path;

		Point<int16_t> left = origin({ 100, 200 }, 65, true);
		require(left.x() == 35 && left.y() == 172, "a shot to the left starts 65 px left of the user, 28 px up");
		Point<int16_t> right = origin({ 100, 200 }, 30, false);
		require(right.x() == 130 && right.y() == 172, "a shot to the right starts in front on the right");

		Point<int16_t> hit = hit_point(left, -300, { -200, -100, 100, 200 });
		require(hit.x() == -150 && hit.y() == 172, "a shot hits the middle of a body across its row");

		hit = hit_point(left, -300, { 0, 80, 100, 200 });
		require(hit.x() == 17, "a body reaching behind the origin is hit between it and the origin");

		hit = hit_point(left, -300, { -200, -100, 100, 150 });
		require(hit.y() == 149, "a body above the row is hit at its bottom");

		hit = hit_point(left, -300, { -400, -350, 100, 200 });
		require(hit.x() == -350, "a body past the end of the row is hit at its near edge");

		require(flight_time({ 0, 0 }, { 300, 40 }) == 453, "a bullet flies 1.5 ms per pixel of the straight path");
		require(hit_delay(left, { -150, 172 }) == 277, "a hit lands 1.5 ms per pixel across");
		require(hit_delay(left, { -150, 100 }) == 277, "only the horizontal distance times the hit");

		require(spread(0, 1) == 0, "a single bullet flies at its target");
		require(spread(0, 2) == -7 && spread(1, 2) == 7, "two bullets fly 7 px either side");
		require(spread(0, 3) == -14 && spread(2, 3) == 14, "three bullets fly 14 px apart");

		require(bullet_delay(2070000, 0, 1) == 120, "throwing stars leave 120 ms apart");
		require(bullet_delay(5021000, 0, 2) == 240, "cash shop stars do too");
		require(bullet_delay(2060000, 0, 1) == 0, "arrows leave together");
		require(bullet_delay(2060000, 3111006, 3) == 180, "Strafe's arrows leave 60 ms apart");
		require(bullet_delay(2330000, 5001003, 1) == 240, "Double Shot's bullets leave 240 ms apart");

		std::cout << "Shot path passed\n";
		return 0;
	}
	catch (const std::exception& e)
	{
		std::cerr << e.what() << '\n';
		return 1;
	}
}
