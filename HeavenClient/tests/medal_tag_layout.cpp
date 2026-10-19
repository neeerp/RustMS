#include "../Character/MedalTagLayout.h"
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
		using namespace ms::MedalTagLayout;

		// Tags 0 to 4: plain caps on either side of 4 px tiles.
		Layout plain = layout({ 6, 15, 6, 2 }, { 4, 15, 0, 2 }, { 6, 15, 0, 2 }, 100);
		require(plain.width == 112 && plain.height == 15, "the tiles cover the text between the caps");
		require(plain.tiles.size() == 25 && plain.tiles.front() == 6 && plain.tiles.back() == 102, "tiles start after the left cap");
		require(plain.tile_y == 0, "every piece shares the top line");
		require(plain.left.x == 0 && plain.left.y == 0, "the left cap is at the corner");
		require(plain.right.x == 106 && plain.right.y == 0, "the right cap ends the tag");
		require(plain.text.x == 6 && plain.text.y == 0, "the text starts with the tiles");
		require(plain.origin_x == 56, "the tag is centred");

		// Tag 5: the text is narrower than its tiles and centred over them.
		Layout gold = layout({ 15, 17, 15, 3 }, { 5, 17, 0, 3 }, { 15, 17, 0, 3 }, 33);
		require(gold.width == 65 && gold.height == 17, "tiles round the text up");
		require(gold.tiles.size() == 7 && gold.tiles.back() == 45, "seven tiles cover 33 px");
		require(gold.right.x == 50, "the right cap follows the last tile");
		require(gold.text.x == 16 && gold.text.y == 1, "the text sits 2 px above the shared line");
		require(gold.origin_x == 32, "an odd width centres to the left");

		// Tag 14: taller caps hang their tiles lower, and the left cap
		// reaches over them.
		Layout tall = layout({ 24, 19, 18, 5 }, { 1, 16, 0, 3 }, { 23, 19, 0, 5 }, 101);
		require(tall.width == 142 && tall.height == 19, "the tallest piece sets the height");
		require(tall.tiles.size() == 101 && tall.tiles.front() == 18, "1 px tiles start at the left cap's cx");
		require(tall.tile_y == 2, "the tiles hang to the shared line");
		require(tall.left.y == 0 && tall.right.x == 119, "the caps keep their places");
		require(tall.text.x == 18 && tall.text.y == 3, "the text follows the shared line");
		require(tall.origin_x == 71, "the tag is centred");

		// A right cap with a smaller cy than the rest still goes at the top.
		Layout lopsided = layout({ 8, 20, 8, 5 }, { 4, 17, 0, 2 }, { 8, 17, 0, 2 }, 8);
		require(lopsided.right.y == 0 && lopsided.tile_y == 3, "the right cap ignores its cy");

		Layout empty = layout({ 6, 15, 6, 2 }, { 4, 15, 0, 2 }, { 6, 15, 0, 2 }, 0);
		require(empty.tiles.empty() && empty.width == 12, "no text needs no tiles");

		require(text("Diligent Explorer Medal") == "Diligent Explorer Medal", "v83 names are kept");
		require(text("Tristan's Successor Medal") == "Tristan's Successor Medal", "only the whole suffix goes");
		require(text("Hero's Medal") == "Hero", "the suffix is removed");
	}
	catch (const std::exception& e)
	{
		std::cerr << "FAIL: " << e.what() << std::endl;
		return 1;
	}

	return 0;
}
