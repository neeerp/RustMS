#pragma once

#include <cstdint>

namespace ms
{
	// What the v83 client checks before an item leaves the inventory for the
	// ground (FUN_004f31a5).
	namespace item_drop
	{
		// Cash items, which include anything with a Cash Shop serial, and
		// Flowerpots (4220xxx) can't be dropped.
		inline bool refused(int32_t item_id, bool cashitem)
		{
			return cashitem || item_id / 1000 == 4220;
		}

		// How many times "can't be taken back" is asked: once for a quest
		// item and once more for an untradable one, so twice for an item
		// that is both.
		inline uint8_t asks(bool questitem, bool untradable)
		{
			return (questitem ? 1 : 0) + (untradable ? 1 : 0);
		}

		// Whether to ask how many to drop. Equipment, cash items, throwing
		// stars and bullets go as one; the server throws a whole stack of
		// stars or bullets.
		inline bool asks_count(int32_t item_id, int16_t count)
		{
			int32_t prefix = item_id / 1000000;
			int32_t kind = item_id / 10000;

			return count >= 2 && prefix >= 2 && prefix <= 4 && kind != 207 && kind != 233;
		}
	}
}
