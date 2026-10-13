#include "../IO/Components/MapTooltipLayout.h"
#include <iostream>
#include <stdexcept>
#include <unordered_map>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

int main()
{
	try
	{
		using namespace ms::map_tooltip;
		using Entries = std::unordered_map<int64_t, std::pair<std::string, std::string>>;

		Entries amherst = { { 1, { "n", "Maria" } }, { 2, { "n", "Pio" } }, { 3, { "n", "Rain" } }, { 4, { "n", "Lucas" } } };
		Entries southperry = { { 5, { "n", "Maple Administrator" } }, { 6, { "n", "Shanks" } }, { 7, { "n", "Biggs" } } };

		Life four = pick_life(amherst);
		Life three = pick_life(southperry);
		require(four.npcs.size() == 4 && four.mobs.empty(), "every NPC is listed and none as a mob");
		require(three.npcs.size() == 3, "a smaller map lists only its own NPCs");

		// Southperry right after Amherst is the same as Southperry alone.
		Size alone = size(120, 18, 30, three, 16);
		size(140, 18, 45, four, 16);
		Size after = size(120, 18, 30, three, 16);
		require(alone.width == after.width && alone.height == after.height, "showing another map first changes nothing");
		require(alone.width == MIN_WIDTH + DESC_MARGIN, "the description margin is added once");
		require(alone.height == 18 + 30 + DESC_GAP + 3 * (16 + LINE_GAP), "name, description, then a line per NPC");

		require(size(200, 18, 0, {}, 16).width == 200, "a long name widens the frame");
		require(size(80, 18, 0, {}, 16).height == 18, "a name alone is one line tall");
		require(size(80, 18, 0, three, 16).height == 18 + 3 * (16 + LINE_GAP) + BOTTOM_PADDING,
			"without a description the life list gets bottom padding");
		require(wrap_width(80) == MIN_WIDTH && wrap_width(200) == 200, "descriptions wrap at the frame's base width");

		Entries crowded;

		for (int64_t id = 0; id < 15; id++)
			crowded[id] = { "m", "Snail" };

		crowded[100] = { "r", "Box" };
		Life capped = pick_life(crowded);
		require(capped.mobs.size() == MAX_LIFE && capped.npcs.empty(), "at most ten mobs, and no other kinds");

		std::cout << "Map tooltip layout passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
