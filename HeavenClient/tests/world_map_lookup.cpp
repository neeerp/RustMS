#include "../IO/UITypes/WorldMapLookup.h"
#include <iostream>
#include <map>
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

// Two continents under WorldMap.img, the first with two regions of its own.
std::string find_npc(std::vector<int32_t> maps)
{
	static const std::map<std::string, std::vector<std::string>> links = {
		{ "WorldMap", { "WorldMap000", "WorldMap010", "WorldMap020" } },
		{ "WorldMap010", { "WorldMap011", "WorldMap012" } }
	};
	static const std::map<std::string, std::set<int32_t>> lists = {
		{ "WorldMap000", { 1 } },
		{ "WorldMap010", { 100, 101, 102 } },
		{ "WorldMap011", { 100, 101 } },
		{ "WorldMap012", { 102 } },
		{ "WorldMap020", { 200 } }
	};

	return ms::world_map::find_npc(maps,
		[&](const std::string& name) {
			auto found = links.find(name);
			return found == links.end() ? std::vector<std::string>() : found->second;
		},
		[&](const std::string& name, int32_t map) {
			auto found = lists.find(name);
			return found != lists.end() && found->second.count(map) > 0;
		});
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

		require(find_npc({}).empty(), "an NPC with no maps is on no world map");
		require(find_npc({ -1 }).empty(), "nor is one NpcLocation.img doesn't list");
		require(find_npc({ 999 }).empty(), "nor one whose maps no world map has");
		require(find_npc({ 100 }) == "WorldMap011", "down to the region that has the map");
		require(find_npc({ 100, 101 }) == "WorldMap011", "a region that has all of them");
		require(find_npc({ 100, 102 }) == "WorldMap010", "maps in two regions stop at their continent");
		require(find_npc({ 100, 200 }) == "WorldMap", "maps on two continents stop at the top");
		require(find_npc({ 200 }) == "WorldMap020", "a continent with no links of its own");
		require(find_npc({ 100, 999 }) == "WorldMap011", "maps no world map has don't count");

		require(ms::world_map::find_npc({ 100 }, [](const std::string&) { return std::vector<std::string>(); },
			[](const std::string&, int32_t) { return true; }) == "WorldMap",
			"WorldMap.img with no links is the one shown");

		std::cout << "World map lookup passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
