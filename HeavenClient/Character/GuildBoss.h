#pragma once

#include "../Net/InPacket.h"

#include <cstdint>
#include <string>

namespace ms
{
	// Ergoth's room in Sharenian (CField_GuildBoss, v83ref b380): a
	// chandelier (the map's healer) hangs over the altar, and a swing at
	// the pulley asks the server to drop it. The server moves it and locks
	// or frees the pulley; the client only draws them.
	namespace guild_boss
	{
		constexpr int32_t FIELD_TYPE = 8;

		// A box by its edges, as a RECT.
		struct Rect
		{
			int32_t left;
			int32_t top;
			int32_t right;
			int32_t bottom;

			// Whether the two overlap, as IntersectRect.
			bool intersects(const Rect& other) const;
		};

		// The map's healer and pulley nodes, as the client reads them
		// (A 5587b3); the rest of healer is the server's.
		struct Rules
		{
			int32_t healer_x = 0;
			int32_t y_min = 0;
			std::string healer;
			int32_t pulley_x = 0;
			int32_t pulley_y = 0;
			std::string pulley;

			// Where a swing pulls the pulley (A 5587b3's RECT).
			Rect pulley_box() const;
		};

		struct State
		{
			// Where the chandelier hangs.
			int32_t healer_y = 0;
			// Non-zero while the pulley is locked.
			uint8_t pulley = 0;
		};

		// The player has been moved to a field of `fieldtype`: the
		// chandelier is back at the top and the pulley free.
		void enter_field(int32_t fieldtype, const Rules& rules);
		bool on_field();

		const Rules& rules();
		State& state();

		// GUILD_BOSS_HEALER_MOVE (0x11F): the chandelier's new height.
		int16_t read_healer_move(InPacket& recv);
		// GUILD_BOSS_PULLEY_STATE (0x120).
		uint8_t read_pulley_state(InPacket& recv);
	}
}
