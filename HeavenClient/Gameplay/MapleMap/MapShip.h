#pragma once

#include "ShipRules.h"

#include "../../Audio/Audio.h"
#include "../../Graphics/Animation.h"

#include <optional>

namespace ms
{
	// The ship a ship field (info/fieldType 2) shows, from its shipObj node
	// (v83 FUN_0054cd0b): one that docks at a station's platform, or the
	// Crimson Balrog's, which comes alongside a ship's deck.
	class MapShip
	{
	public:
		MapShip(nl::node src);

		MapShip();

		// The map's shipKind, or -1 if it has no ship.
		int32_t get_kind() const;

		// CONTI_MOVE: the trip changed.
		void move(uint8_t target, uint8_t event);

		// CONTI_STATE: where the trip is, on entering the map.
		void show_state(uint8_t state, uint8_t flag);

		void update();

		// A docking ship sails behind all on the map (z 0xC0000064)...
		void draw_behind(double viewx, double viewy, float alpha) const;

		// ...and the Balrog's comes above it (z 0xC0061378).
		void draw_above(double viewx, double viewy, float alpha) const;

	private:
		void act(ship::Action action);

		void draw(double viewx, double viewy, float alpha) const;

		std::optional<ship::Motion> motion;
		Animation animation;
		// A docking ship whistles as it comes and goes.
		Sound whistle;
	};
}
