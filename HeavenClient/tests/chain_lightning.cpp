#include "../Gameplay/Combat/ChainLightning.h"
#include <iostream>
#include <stdexcept>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

namespace
{
	using ms::chain_lightning::Mob;

	// A 40 px wide, 60 px tall mob standing at (x, y).
	Mob mob(int32_t oid, int16_t x, int16_t y, uint32_t entry)
	{
		return { oid, { x, y }, { static_cast<int16_t>(x - 20), static_cast<int16_t>(x + 20), static_cast<int16_t>(y - 60), y }, entry };
	}
}

int main()
{
	try
	{
		using namespace ms;
		using namespace chain_lightning;

		Mob first = mob(1, 0, 0, 0);

		// It jumps to the nearest mob ahead, then on from that one.
		std::vector<int32_t> hit = chain(first, { first, mob(2, 200, 0, 1), mob(3, 100, 0, 2) }, 6, false);
		require(hit == std::vector<int32_t>({ 1, 3, 2 }), "it jumps to the nearer mob first, then on");

		// Facing left, it only jumps left.
		hit = chain(first, { first, mob(2, 100, 0, 1), mob(3, -100, 0, 2) }, 6, true);
		require(hit == std::vector<int32_t>({ 1, 3 }), "it jumps the way the user faces");

		// A body reaching 150 px ahead is in reach.
		hit = chain(first, { first, mob(2, 169, 0, 1) }, 6, false);
		require(hit == std::vector<int32_t>({ 1, 2 }), "a body that starts 149 px ahead is reached");
		hit = chain(first, { first, mob(2, 170, 0, 1) }, 6, false);
		require(hit == std::vector<int32_t>({ 1 }), "one that starts 150 px ahead is not");

		// Above or below, within 150 px.
		hit = chain(first, { first, mob(2, 50, 209, 1) }, 6, false);
		require(hit == std::vector<int32_t>({ 1, 2 }), "a mob below whose head is 149 px down is reached");
		hit = chain(first, { first, mob(2, 50, -150, 1) }, 6, false);
		require(hit == std::vector<int32_t>({ 1 }), "a mob 150 px up is not");

		// It stops at the skill's mob count.
		hit = chain(first, { first, mob(2, 50, 0, 1), mob(3, 100, 0, 2), mob(4, 150, 0, 3) }, 3, false);
		require(hit == std::vector<int32_t>({ 1, 2, 3 }), "it hits as many mobs as the skill says");

		// No further than 400 px ahead of the first mob.
		hit = chain(first, { first, mob(2, 140, 0, 1), mob(3, 280, 0, 2), mob(4, 400, 0, 3), mob(5, 540, 0, 4) }, 6, false);
		require(hit == std::vector<int32_t>({ 1, 2, 3, 4 }), "it reaches 400 px ahead of the first mob, and no more");

		// It doesn't jump back to a mob it has hit.
		hit = chain(first, { first, mob(2, 100, 0, 1), mob(3, 100, 10, 2) }, 6, false);
		require(hit == std::vector<int32_t>({ 1, 2, 3 }), "a mob it has hit is passed over");

		// On a tie, the mob that entered the map first.
		hit = chain(first, { first, mob(3, 100, 0, 5), mob(2, 100, 0, 1) }, 2, false);
		require(hit == std::vector<int32_t>({ 1, 2 }), "the earlier mob wins a tie");

		// It looks at only the first 10 mobs in reach, by when they entered.
		std::vector<Mob> crowd = { first };

		for (int32_t i = 0; i < 10; i++)
			crowd.push_back(mob(10 + i, 140, 0, 1 + i));

		crowd.push_back(mob(99, 30, 0, 50));
		hit = chain(first, crowd, 2, false);
		require(hit == std::vector<int32_t>({ 1, 10 }), "a close mob that entered later is passed over");

		// The bolt leaves the user's hand, and meets a mob in the middle.
		Point<int16_t> hand = start({ 100, 200 }, true);
		require(hand.x() == 75 && hand.y() == 172, "the bolt leaves 25 px in front of the user, 28 px up");
		Point<int16_t> centre = middle({ -21, 20, -61, 0 });
		require(centre.x() == 0 && centre.y() == -30, "the middle rounds toward zero");

		// One ball per 48 px and one more, 48 px apart around the middle.
		std::vector<Ball> balls = bolt({ 0, 0 }, { 100, 0 });
		require(balls.size() == 3, "a 100 px bolt has three balls");
		require(balls[0].position.x() == 2 && balls[1].position.x() == 50 && balls[2].position.x() == 98,
				"they lie 48 px apart around the middle");
		require(balls[0].degrees == 0, "a level bolt lies flat");

		balls = bolt({ 0, 0 }, { 0, 100 });
		require(balls[0].degrees == 90 && balls[0].position.x() == 0 && balls[0].position.y() == 2,
				"a bolt straight down stands up");
		require(bolt({ 0, 100 }, { 0, 0 })[0].degrees == 270, "and so does one straight up");

		balls = bolt({ 100, 100 }, { 0, 0 });
		require(balls.size() == 3 && balls[0].degrees == 45, "a bolt drawn right to left turns the same way");
		require(balls[1].position.x() == 50 && balls[1].position.y() == 50, "its middle ball is in the middle");
		require(balls[0].position.x() == 16 && balls[0].position.y() == 16, "the others lie along it");

		require(bolt({ 0, 0 }, { 47, 0 }).size() == 1, "a short bolt is a single ball");

		std::cout << "Chain Lightning passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
