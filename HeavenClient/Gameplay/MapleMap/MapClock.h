#pragma once

#include "ClockRules.h"

#include "../../Graphics/Texture.h"

#include <array>

namespace ms
{
	// The wall clock on a station's sign, from a map's `clock` node: a box
	// in the map whose time CLOCK (147) kind 1 sets (v83 0x52bc72).
	class MapClock
	{
	public:
		MapClock(nl::node src);

		MapClock();

		bool is_present() const;

		// The time of day, on a 24 hour dial.
		void set(uint8_t hour, uint8_t minute, uint8_t second);

		void update();

		// Above all on the map but a Balrog's ship (z 0xC0041EE2).
		void draw(double viewx, double viewy) const;

	private:
		bool present = false;
		Point<int16_t> position;
		wall_clock::Layout layout = {};
		wall_clock::Clock clock;
		std::array<Texture, 10> digits;
		Texture colon;
		Texture am;
		Texture pm;
	};
}
