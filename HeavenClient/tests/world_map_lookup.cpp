#include "../IO/UITypes/WorldMapLookup.h"
#include <iostream>
#include <set>
#include <stdexcept>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

std::string find(std::set<std::string> listing)
{
	return ms::world_map::find([&](const std::string& name) { return listing.count(name) > 0; });
}

int main()
{
	try
	{
		require(ms::world_map::name(0) == "WorldMap000" && ms::world_map::name(142) == "WorldMap142",
			"names have three digits");

		require(find({}).empty(), "a field no world map lists has none");
		require(find({ "WorldMap" }).empty(), "WorldMap.img is never chosen");
		require(find({ "WorldMap011" }).empty(), "a sub-map alone isn't looked at");

		require(find({ "WorldMap010" }) == "WorldMap010", "a top world map");
		require(find({ "WorldMap010", "WorldMap012", "WorldMap013" }) == "WorldMap012",
			"the first sub-map listing it wins");
		require(find({ "WorldMap010", "WorldMap020" }) == "WorldMap020",
			"the tenth after a top world map counts as one of its own");
		require(find({ "WorldMap020", "WorldMap040" }) == "WorldMap020", "the first top world map wins");
		require(find({ "WorldMap100" }) == "WorldMap100", "WorldMap100 is the last top world map");

		require(find({ "WorldMap141" }) == "WorldMap141", "WorldMap140 to 142 on their own");
		require(find({ "WorldMap010", "WorldMap140" }) == "WorldMap140", "they beat a top world map");
		require(find({ "WorldMap010", "WorldMap011", "WorldMap140" }) == "WorldMap011",
			"a sub-map beats them");

		std::cout << "World map lookup passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
