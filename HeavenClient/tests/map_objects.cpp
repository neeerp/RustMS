#include "../Gameplay/MapleMap/MapObjects.h"
#include <iostream>
#include <stdexcept>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

namespace
{
	class Dummy : public ms::MapObject
	{
	public:
		Dummy(int32_t oid) : MapObject(oid)
		{}

		void draw(double, double, float) const override
		{}
	};

	std::unique_ptr<ms::MapObject> dummy(int32_t oid)
	{
		return std::make_unique<Dummy>(oid);
	}
}

int main()
{
	try
	{
		ms::MapObjects objects;

		// Melee attacks pick mobs in the order they entered the map, not by oid.
		objects.add(dummy(30));
		objects.add(dummy(10));
		objects.add(dummy(20));
		require(objects.entry_order(30) < objects.entry_order(10), "the first mob entered first");
		require(objects.entry_order(10) < objects.entry_order(20), "a later oid can enter earlier");

		// A mob that enters again under the same oid keeps its place.
		objects.add(dummy(30));
		require(objects.entry_order(30) < objects.entry_order(10), "re-entering keeps the place");

		// A mob that was removed and spawns again goes to the back.
		objects.remove(30);
		objects.add(dummy(30));
		require(objects.entry_order(20) < objects.entry_order(30), "a respawned mob is last");

		objects.clear();
		objects.add(dummy(40));
		require(objects.entry_order(40) < objects.entry_order(10), "cleared mobs have no place");
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
