#include "../Gameplay/MapleMap/MapObjects.h"
#include <iostream>
#include <stdexcept>
#include <vector>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

namespace
{
	std::vector<int32_t> drawn;

	class Dummy : public ms::MapObject
	{
	public:
		Dummy(int32_t oid, bool restack = false) : MapObject(oid), restack(restack)
		{}

		void draw(double, double, float) const override
		{
			drawn.push_back(get_oid());
		}

		int8_t update(const ms::Physics &) override
		{
			return gone ? -1 : layer;
		}

		int8_t get_layer() const override
		{
			return layer;
		}

		bool take_restack() override
		{
			bool taken = restack;
			restack = false;

			return taken;
		}

		bool restack;
		bool gone = false;
		int8_t layer = 0;
	};

	std::unique_ptr<ms::MapObject> dummy(int32_t oid)
	{
		return std::make_unique<Dummy>(oid);
	}

	std::unique_ptr<ms::MapObject> restacker(int32_t oid)
	{
		return std::make_unique<Dummy>(oid, true);
	}

	std::vector<int32_t> drawn_on(const ms::MapObjects &objects, int layer)
	{
		drawn.clear();
		objects.draw(static_cast<ms::Layer::Id>(layer), 0, 0, 0);

		return drawn;
	}

	using Order = std::vector<int32_t>;

	Dummy &dummy_in(ms::MapObjects &objects, int32_t oid)
	{
		return static_cast<Dummy &>(*objects.get(oid).get());
	}

	void draw_order()
	{
		ms::Physics physics;
		ms::MapObjects objects;

		// A later mob sets its z and draws over the earlier ones, whatever the
		// oids: Scrooge comes out at the snowman's feet and is seen in front.
		objects.add(restacker(70));
		objects.add(restacker(30));
		objects.add(restacker(50));
		require(drawn_on(objects, 0) == Order{ 70, 30, 50 }, "later mobs draw on top");

		// One that changes foothold goes over the rest.
		dummy_in(objects, 70).restack = true;
		objects.update(physics);
		require(drawn_on(objects, 0) == Order{ 30, 50, 70 }, "a new foothold goes on top");

		// A new layer goes under its equals when nothing sets its z.
		objects.add(dummy(90));
		objects.add(dummy(80));
		require(drawn_on(objects, 0) == Order{ 80, 90, 30, 50, 70 }, "a plain object goes under");

		// A change of layer alone keeps an object's place against its new equals.
		dummy_in(objects, 90).layer = 2;
		dummy_in(objects, 30).layer = 2;
		objects.update(physics);
		require(drawn_on(objects, 2) == Order{ 90, 30 }, "a move of layer keeps the stack");
		require(drawn_on(objects, 0) == Order{ 80, 50, 70 }, "it leaves the old layer");

		// One that an update removes leaves no ghost, so its oid can come back.
		dummy_in(objects, 50).gone = true;
		objects.update(physics);
		objects.add(restacker(50));
		require(drawn_on(objects, 0) == Order{ 80, 70, 50 }, "a removed oid draws once");

		// Entering again under the same oid draws it once too.
		objects.add(restacker(70));
		require(drawn_on(objects, 0) == Order{ 80, 50, 70 }, "a repeated oid draws once");

		objects.remove(50);
		require(drawn_on(objects, 0) == Order{ 80, 70 }, "a removed object is not drawn");

		objects.clear();
		objects.add(dummy(1));
		require(drawn_on(objects, 0) == Order{ 1 }, "clearing leaves nothing behind");
		require(drawn_on(objects, 2).empty(), "clearing empties every layer");
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

		draw_order();
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
