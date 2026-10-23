#pragma once

#include "MapSummons.h"

#include "../../Graphics/Text.h"

#include <array>
#include <optional>

namespace ms
{
	// The guide that follows a new player through the tutorial, with its
	// balloons and hint pictures (v83 SPAWN_GUIDE 223, TALK_GUIDE 224,
	// report b099). It is a summon only the player's client knows of: it
	// follows them and sends nothing.
	class Guide
	{
	public:
		// SPAWN_GUIDE with 1: anew at the player's feet, playing `summoned`.
		void spawn(int32_t skill, Point<int16_t> position);

		// SPAWN_GUIDE with 0, or level 11: gone at once, without `die`.
		void remove();

		bool is_present() const;

		// It comes along to a new map, appearing again where the player is
		// (0x949a3b).
		void respawn(Point<int16_t> position);

		// The new guide has no balloon or hint (0x949a3b makes it anew). This
		// runs when the old map is left, so what TALK_GUIDE says while the
		// new one loads stays.
		void hush();

		// TALK_GUIDE mode 0: `text` wrapped at `width` px for `duration` ms
		// (FUN_007a7b46).
		void say(const std::string &text, int32_t width, int32_t duration);

		// TALK_GUIDE's other modes: UI/tutorial.img/<id> for `duration` ms
		// (FUN_007a9283), then `say` if `talks`.
		void hint(int32_t id, int32_t duration, bool talks);

		void update(const Physics &physics, Point<int16_t> owner, bool held);

		// In front of the whole map.
		void draw(double viewx, double viewy, float alpha) const;

	private:
		void draw_balloon(Point<int16_t> foot) const;

		enum class Speech
		{
			NONE,
			BALLOON,
			HINT
		};

		// The frame pieces of ChatBalloon.img/tutorial.
		enum Piece : uint8_t
		{
			NW, N, NE, W, C, E, SW, S, SE, ARROW, NUM_PIECES
		};

		int32_t skill = 0;
		int16_t height = 0;
		std::optional<Summon> body;
		Randomizer random;

		Speech speech = Speech::NONE;
		int32_t elapsed = 0;
		int32_t duration = 0;
		int16_t width = 0;
		Text text;
		std::array<Texture, NUM_PIECES> pieces;
		Animation picture;
	};
}
