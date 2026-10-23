#pragma once

#include "SetEffectRules.h"

#include "Look/CharLook.h"

#include "../Graphics/Animation.h"

#include <nlnx/node.hpp>

#include <optional>

namespace ms
{
	// The effect of the set of items a character wears, from Effect/SetEff.img (v83 FUN_009393fc): an
	// animation that loops while they wear it, and a background that shows while they stand.
	class SetEffect
	{
	public:
		// Show the set `look` and the pets out complete, if it isn't shown already; `standing` is whether the
		// character stands still.
		void update(const CharLook &look, const std::array<int32_t, 3> &pets, bool standing);

		// The set shown, or -1.
		int32_t get() const;

		// What goes behind the avatar, or in front of it. `feet` and `face` are on the screen.
		void draw(bool in_front, Point<int16_t> feet, Point<int16_t> face, bool facing_right, float alpha) const;

	private:
		// `pos` 1 is on the face, any other on the feet; `fixed` never mirrors; a negative `z` is behind
		// (FUN_00941417).
		struct Layer
		{
			Animation animation;
			bool at_face;
			bool fixed;
			bool in_front;
			int16_t loop_from;
		};

		static std::optional<Layer> load(nl::node src);

		// Show set `id`'s effect from its start, or none with -1.
		void set(int32_t id);

		// What the set was last matched against: a morph or a ghost completes none.
		struct Worn
		{
			set_effect::Parts parts = {};
			int32_t cash_weapon = 0;
			std::array<int32_t, 3> pets = {};
			bool morphed = false;

			bool operator==(const Worn &other) const
			{
				return parts == other.parts && cash_weapon == other.cash_weapon && pets == other.pets &&
					   morphed == other.morphed;
			}
		};

		Worn worn;
		bool matched = false;
		int32_t id = -1;
		std::optional<Layer> effect;
		std::optional<Layer> background;
		set_effect::Background shown;
		float opacity = 1.0f;
		int64_t now = 0;
	};
}
