#pragma once

#include "Layer.h"

#include "../Combat/DamageNumber.h"

#include "../../Audio/Audio.h"
#include "../../Character/Snowball.h"
#include "../../Graphics/Animation.h"

#include <array>
#include <list>

namespace ms
{
	// What a Snowball field (info/fieldType 1) shows: the two balls, the
	// snowmen with their HP bars, and the hits on them (CField_SnowBall,
	// v83ref b168). The game itself is snowball::state(), which the packets
	// set before the field loads.
	class MapSnowball
	{
	public:
		MapSnowball(nl::node src);

		MapSnowball();

		// Whether this is a Snowball field and the player is still on it.
		bool is_active() const;

		// The due hits, then the balls' roll while the game runs (A 574df1).
		void update();

		// The balls and snowmen are just behind layer 3's tiles and objects
		// (z 0xc0015f90, page 3's 90000 less 100), so in front of layers 0
		// to 2.
		void draw(Layer::Id layer, double viewx, double viewy, float alpha) const;

		// The hits' numbers are with the others', and the HP bars in front
		// of the whole field (z 2, A 574294).
		void draw_front(double viewx, double viewy, float alpha) const;

	private:
		// A hit due now (A 5754ca's node, shown in A 574df1).
		void show(const snowball::Hit& hit);

		void draw_bar(size_t lane, int16_t x, int16_t y) const;

		bool active = false;
		std::array<Texture, 2> balls;
		std::array<Animation, 2> snowmen;
		// The HP each bar was last drawn with: at the field's start, and at
		// each hit on its snowman.
		std::array<int32_t, 2> bars = { 100, 100 };
		std::list<DamageNumber> numbers;
		Sound hit_sound;
	};
}
