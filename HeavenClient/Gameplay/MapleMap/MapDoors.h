#pragma once

#include "Layer.h"

#include "../Physics/Physics.h"
#include "../../Graphics/Animation.h"

#include <map>
#include <optional>
#include <vector>

namespace ms
{
	// The Mystic Doors on a map: those standing in the field, which lead to
	// town, and those standing at a town's door portals, which lead back.
	class MapDoors
	{
	public:
		// Where a door stands, on the ground, and the layer of that ground.
		struct Spot
		{
			Point<int16_t> position;
			Layer::Id layer;
		};

		// Load the door's animations, once the NX files are open.
		static void init();

		// Where a door at `position` stands: on the ground below (x, y - 5),
		// or where it is if there's none (FUN_00a45585).
		static Spot place(const Physics &physics, Point<int16_t> position);

		// A door appears in the field (SPAWN_DOOR, FUN_007bd6c6). One just
		// cast opens (cDoor) and stands 1.8 s later (FUN_007bd262); one cast
		// before stands at once (mDoor in its Frame).
		void spawn(int32_t owner, Point<int16_t> position, bool just_cast);

		// A door goes (REMOVE_DOOR, FUN_007be064), fading out over a second
		// if `fade`.
		void remove(int32_t owner, bool fade);

		// The doors are placed here rather than on SPAWN_DOOR, which can
		// come before the field's footholds are loaded.
		void update(const Physics &physics);

		// Draw the doors on a layer: the field's, and the town's at `town`.
		void draw(Layer::Id layer, const std::vector<Spot> &town, Point<int16_t> viewpos, float alpha) const;

		// The owner of the field door the player stands in, or 0
		// (FUN_007bd594).
		int32_t find(Point<int16_t> playerpos) const;

		void clear();

	private:
		struct Door
		{
			Point<int16_t> position;
			std::optional<Spot> spot;
			Animation opening;
			Animation standing;
			Animation frame;
			// The ms since it was cast, while it opens.
			int16_t age = 0;
			bool opened = false;
			// The ms left of its fade, once removed.
			int16_t fading = -1;
		};

		static void draw_standing(const Animation &standing, const Animation &frame, Point<int16_t> position,
								  float opacity, float alpha);

		// Skill.nx 231.img/skill/2311002: cDoor, mDoor and Frame.
		static Animation opening_template;
		static Animation standing_template;
		static Animation frame_template;

		// The town's doors share one animation.
		Animation town_standing;
		Animation town_frame;
		std::map<int32_t, Door> doors;
	};
}
