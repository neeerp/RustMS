#include "../Gameplay/Combat/HitPoint.h"

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
		using namespace ms::hit_point;

		// Left, right, top, bottom.
		Rectangle<int16_t> body{ 80, 120, 140, 200 };

		require(centre({ 100, 200, 150, 170 }, body) == Point<int16_t>(110, 160), "the middle of the overlap");
		require(centre({ 0, 50, 150, 170 }, body) == Point<int16_t>(80, 160), "a short attack touches the body");
		require(centre({ 130, 180, 150, 170 }, body) == Point<int16_t>(119, 160), "from either side");

		Point<int16_t> mob{ 100, 200 };
		Point<int16_t> hit{ 110, 160 };

		auto feet = place(1, body, mob, hit);
		require(feet && feet->anchor == Anchor::FEET && feet->offset == Point<int16_t>(), "pos 1 at the feet");

		auto head = place(2, body, mob, hit);
		require(head && head->anchor == Anchor::HEAD && head->offset == Point<int16_t>(), "pos 2 on the head");

		auto middle = place(3, body, mob, hit);
		require(middle && middle->anchor == Anchor::FEET && middle->offset == Point<int16_t>(0, -30),
				"pos 3 in the middle of the body");

		auto struck = place(4, body, mob, hit);
		require(struck && struck->anchor == Anchor::FEET && struck->offset == Point<int16_t>(10, -40),
				"pos 4 where the hit landed, following the mob");

		auto field = place(0, body, mob, hit);
		require(field && field->anchor == Anchor::FIELD && field->offset == hit, "no pos: where the hit landed");

		auto clamped = place(0, body, mob, { 300, 100 });
		require(clamped && clamped->offset == Point<int16_t>(120, 140), "kept on the body, edges included");

		Rectangle<int16_t> hollow{ 100, 100, 150, 200 };

		require(!place(0, hollow, mob, hit), "no body, no effect");
		require(!place(3, hollow, mob, hit), "nor in its middle");
		require(place(1, hollow, mob, hit) && place(2, hollow, mob, hit) && place(4, hollow, mob, hit),
				"but the feet and head still show one");

		std::cout << "hit point tests passed\n";
	}
	catch (const std::exception &error)
	{
		std::cerr << error.what() << '\n';

		return 1;
	}

	return 0;
}
