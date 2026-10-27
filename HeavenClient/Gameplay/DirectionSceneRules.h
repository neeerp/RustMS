#pragma once

#include <cstdint>
#include <string>

// The numbers behind a scene the server asks for with SHOW_ITEM_GAIN_INCHAT mode 0x12 ("Effect/Direction2.img/open/
// back0"), which v83 reads with FUN_00439e00 and runs with FUN_0043a8c4 (report b201). Each part holds events: what to
// show and when, or which field to ask for when the scene ends.
namespace ms::direction
{
	struct Event
	{
		enum Type : int32_t
		{
			// A picture from `visual` ("Effect/Direction2.img/effect/open/back0"), drawn once.
			VISUAL = 0,
			// The field the scene leads to, which the client asks the server for.
			FIELD = 2,
			// Only its `sound`, which a visual also plays.
			SOUND = 5
		};

		int32_t type = VISUAL;
		// When it begins, in ms after the part came; each counts from then, not from the event before.
		int32_t start = 0;
		std::string visual;
		std::string sound;
		int32_t field = 0;
		// Where it is drawn in the field, the player's position when the part came unless the event says; with an (x1,
		// y1) it moves there in `duration` ms.
		int16_t x = 0;
		int16_t y = 0;
		int16_t x1 = 0;
		int16_t y1 = 0;
		int32_t duration = 0;
		// Higher in front, all at the field's effect layer (0xc00614a4 + z).
		int32_t z = 0;
	};

	struct Position
	{
		int16_t x;
		int16_t y;
	};

	// Where a visual is `elapsed` ms after it began: moving in a straight line from (x, y) to (x1, y1) for its
	// duration, then staying there. v83 moves only those whose x1 or y1 isn't 0.
	inline Position position(const Event &event, int32_t elapsed)
	{
		if (event.x1 == 0 && event.y1 == 0)
			return { event.x, event.y };

		if (event.duration <= 0 || elapsed >= event.duration)
			return { event.x1, event.y1 };

		if (elapsed <= 0)
			return { event.x, event.y };

		auto along = [&](int16_t from, int16_t to) {
			return static_cast<int16_t>(from + static_cast<int64_t>(to - from) * elapsed / event.duration);
		};

		return { along(event.x, event.x1), along(event.y, event.y1) };
	}
}
