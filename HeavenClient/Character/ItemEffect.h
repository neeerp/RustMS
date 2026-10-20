#pragma once

#include "ItemEffectRules.h"

#include "Look/CharLook.h"

#include "../Graphics/Animation.h"

#include <nlnx/node.hpp>

#include <deque>
#include <optional>
#include <string>
#include <vector>

namespace ms
{
	// The effect of a 501xxxx cash or 429xxxx etc item a character shows (v83 FUN_0093b6a7): an animation on the
	// face or the feet, up to five followers trailing the face, or fading ghosts of the avatar. It loops until it
	// stops.
	class ItemEffect
	{
	public:
		// Show `item_id`'s effect from its start, or none with 0.
		void set(int32_t item_id);
		int32_t get() const;

		// Start it over where the character is now, as on a new field.
		void restart();

		// `action` is the body action or stance under way; `feet` and `face` are in the world.
		void update(const std::string &now_action, Point<int16_t> feet, Point<int16_t> face, const CharLook &look);

		// The ghosts, behind the rest of the character.
		void draw_ghosts(Point<int16_t> view) const;

		// What goes behind the avatar, or in front of it. `feet` and `face` are on the screen; `view` turns the
		// world into the screen.
		void draw(bool in_front, Point<int16_t> feet, Point<int16_t> face, bool facing_right, Point<int16_t> view,
				  float alpha) const;

	private:
		// `pos` 1 is on the face, any other on the feet; `fixed` never mirrors; a negative `z` is behind
		// (FUN_00941417).
		struct Layer
		{
			Animation animation;
			bool at_face;
			bool fixed;
			bool in_front;
		};

		// One of `effect/0` to `effect/4` (FUN_0093c218).
		struct Follower
		{
			Animation animation;
			bool in_front;
			int32_t loose;
			item_effect::Trail x;
			item_effect::Trail y;
			Point<int16_t> at;
		};

		struct Ghost
		{
			CharLook look;
			Point<int16_t> at;
			int64_t born;
			int32_t alpha;
			int32_t delay;
		};

		void load(const std::string &key);

		int32_t item_id = 0;
		nl::node effect;
		bool keyed = false;
		std::string action;
		std::optional<Layer> layer;
		std::vector<Follower> followers;

		bool spectrum = false;
		int32_t interval = 100;
		int32_t delay = 1000;
		int32_t ghost_alpha = 0x80;
		std::deque<Ghost> ghosts;
		int64_t next_ghost = 0;
		int64_t now = 0;
	};
}
