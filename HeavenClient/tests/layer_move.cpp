#include "../Gameplay/MapleMap/LayerMove.h"
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
		using namespace ms::layer_move;
		using Offset = std::pair<int32_t, int32_t>;

		Keys still;
		require(offset(still, 1234) == Offset(0, 0), "no type, no swing");

		// The Keep's menhir 0: from where it stands, to the right first.
		Keys menhir{ 1, 450, 100, 9500, 0, true };
		require(offset(menhir, 0) == Offset(0, 0), "starts where it stands");
		require(offset(menhir, 2375) == Offset(450, 0), "right at a quarter");
		require(offset(menhir, 4750) == Offset(0, 0), "back at a half");
		require(offset(menhir, 7125) == Offset(-450, 0), "left at three quarters");
		require(offset(menhir, 9500) == Offset(0, 0), "round again");

		// Its menhir 1 swings left first, and its menhir 2 down first, by their negative width and height.
		require(offset({ 1, -300, 100, 8000, 0, true }, 2000) == Offset(-300, 0), "negative width");
		require(offset({ 2, 100, -360, 18000, 0, true }, 4500) == Offset(0, 360), "negative height");
		require(offset({ 2, 100, 40, 4000, 0, true }, 1000) == Offset(0, -40), "centred up first");

		// Without centerStart it starts from one end of the swing, about where it was placed.
		Keys sign{ 2, 100, 5, 2000, 0, false };
		require(offset(sign, 0) == Offset(0, 5), "starts at the end");
		require(offset(sign, 500) == Offset(0, 0), "through the middle");
		require(offset(sign, 1000) == Offset(0, -5), "to the other end");
		require(offset({ 1, 50, 100, 1600, 0, false }, 0) == Offset(50, 0), "along x too");
		require(offset({ 1, 50, 100, 1600, 0, false }, 800) == Offset(-50, 0), "and back");

		// A negative period only turns the other way, which a swing without centerStart can't show.
		require(offset({ 2, 100, 20, -4000, 0, false }, 1000) == Offset(0, 0), "same middle");
		require(offset({ 2, 100, 40, -4000, 0, true }, 1000) == Offset(0, 40), "centred down first");

		// The saws' ellipse, with y scaled off x by whole px.
		Keys saw{ 3, 100, 80, 4000, 0, false };
		require(offset(saw, 0) == Offset(100, 0), "starts at the right");
		require(offset(saw, 1000) == Offset(0, 80), "then the bottom");
		require(offset(saw, 2000) == Offset(-100, 0), "the left");
		require(offset(saw, 3000) == Offset(0, -80), "the top");
		require(offset(saw, 500) == Offset(71, 56), "71 * 80 / 100 is 56");
		require(offset({ 3, 100, 80, 4000, 0, true }, 0) == Offset(100, 0), "an ellipse ignores centerStart");
		require(offset({ 3, -180, -120, 6000, 0, false }, 1500) == Offset(0, -120), "both negative");

		// Width and height that come to nothing leave it, as a period of 0 leaves it at the start.
		require(offset({ 3, 0, 0, 5000, 6000, false }, 1000) == Offset(0, 0), "a spin only");
		require(offset({ 1, 0, 100, 5000, 0, false }, 1000) == Offset(0, 0), "no width");
		require(offset({ 1, 50, 100, 0, 0, false }, 1000) == Offset(50, 0), "no period");

		require(spun(still, 1000) == 0.0f, "no spin");
		require(std::abs(spun({ 0, 100, 100, 5000, 1500, false }, 375) - 1.5707963f) < 0.0001f, "a quarter turn");
		require(std::abs(spun({ 0, 100, 100, 5000, -1500, false }, 375) + 1.5707963f) < 0.0001f, "the other way");
		require(std::abs(spun({ 0, 100, 100, 5000, 1500, false }, 1875) - 1.5707963f) < 0.0001f, "and round");

		std::cout << "Layer move passed" << std::endl;
	}
	catch (const std::exception &error)
	{
		std::cout << error.what() << std::endl;
		return 1;
	}

	return 0;
}
