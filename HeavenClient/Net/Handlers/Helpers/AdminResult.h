#pragma once

#include "../../InPacket.h"

#include <cstdint>
#include <string>

namespace ms
{
	// ADMIN_RESULT (144): how a GM's slash command went, read as v83 reads
	// it (FUN_005352e9, report b175 s5).
	struct AdminResult
	{
		enum class Kind
		{
			// Nothing to show.
			NONE,
			// A line in chat of v83's type `chat`.
			LINE,
			// The hide state (mode 0x10).
			HIDDEN,
			// A hired merchant found on map `map_id`, whose name follows `line`.
			MERCHANT_MAP,
			// The minimap off or on (modes 0x1B and 0x1C).
			MINIMAP_OFF,
			MINIMAP_ON
		};

		Kind kind = Kind::NONE;
		std::string line;
		// v83's chat type: 9, 0xb or 0xc.
		uint8_t chat = 0;
		bool hidden = false;
		int32_t map_id = 0;

		static AdminResult read(InPacket& recv);
	};
}
