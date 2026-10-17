#include "../Gameplay/Combat/SpellArea.h"

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

int main()
{
	try
	{
		using namespace ms;
		using namespace ms::spell_area;

		require(kind(2121001) == Kind::BIG_BANG && kind(2221001) == Kind::BIG_BANG && kind(2321001) == Kind::BIG_BANG,
				"each Big Bang blasts");
		require(kind(2111002) == Kind::EXPLOSION, "and Explosion");
		require(kind(2121007) == Kind::TILES && kind(2221007) == Kind::TILES && kind(2321008) == Kind::TILES
				&& kind(12111003) == Kind::TILES, "the ultimates cover the ground");
		require(kind(2001005) == Kind::NONE && kind(2121003) == Kind::NONE, "other spells leave nothing");

		require(same(big_bang(0, { 100, 200 }), { 0, 200, 85, 235 }), "an uncharged Big Bang, 40 px up");
		require(same(big_bang(33, { 100, 200 }), { 0, 200, 85, 235 }), "grows in whole pixels");
		require(same(big_bang(1000, { 100, 200 }), { -20, 220, 70, 250 }), "a charged one is bigger");

		require(same(explosion({ -200, 200, -150, 150 }, { 100, 200 }), { -100, 300, 10, 310 }),
				"Explosion's lt/rb, 40 px up");

		Rectangle<int16_t> lopsided{ -400, 100, -350, 250 };

		require(same(tiles(lopsided, { 1000, 200 }, true), { 600, 1100, -150, 450 }), "tiles facing left");
		require(same(tiles(lopsided, { 1000, 200 }, false), { 900, 1400, -150, 450 }), "mirrored facing right");

		require(burn_time(2121007) == 3000 && burn_time(12111003) == 3000, "meteors burn 3 s");
		require(burn_time(2221007) == 2760 && burn_time(2321008) == 2760, "Blizzard and Genesis 2.76 s");

		Spread spread({ 0, 240, 0, 120 });

		require(same(spread.widen(), { 100, 140, 50, 70 }), "the first wave lands in the middle sixth");
		require(same(spread.widen(), { 75, 165, 38, 83 }), "each next wider");
		require(same(spread.widen(), { 56, 184, 28, 92 }), "and wider");

		std::cout << "spell area tests passed\n";
	}
	catch (const std::exception &error)
	{
		std::cerr << error.what() << '\n';

		return 1;
	}

	return 0;
}
