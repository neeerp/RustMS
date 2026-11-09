#include "Audio/RecentTracks.h"

#include <cassert>
#include <string>
#include <vector>

using ms::RecentTracks;

static void keeps_the_last_three()
{
	RecentTracks recent(3);

	assert(recent.use("Bgm00/a").empty());
	assert(recent.use("Bgm00/b").empty());
	assert(recent.use("Bgm00/c").empty());

	std::vector<std::string> dropped = recent.use("Bgm00/d");
	assert(dropped == std::vector<std::string>{ "Bgm00/a" });
	assert(recent.kept().size() == 3);
	assert(recent.kept().front() == "Bgm00/d");
}

static void a_track_used_again_moves_to_the_front()
{
	RecentTracks recent(3);
	recent.use("town");
	recent.use("field");
	recent.use("dungeon");

	// Back to the town: nothing is dropped, and the field is now the oldest.
	assert(recent.use("town").empty());
	assert(recent.kept().front() == "town");
	assert(recent.use("shop") == std::vector<std::string>{ "field" });
	assert(recent.kept().size() == 3);
}

static void walking_between_two_maps_drops_nothing()
{
	RecentTracks recent(3);

	for (int i = 0; i < 10; i++)
	{
		assert(recent.use("town").empty());
		assert(recent.use("field").empty());
	}

	assert(recent.kept().size() == 2);
}

int main()
{
	keeps_the_last_three();
	a_track_used_again_moves_to_the_front();
	walking_between_two_maps_drops_nothing();

	return 0;
}
