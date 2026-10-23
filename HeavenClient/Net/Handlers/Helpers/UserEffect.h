#pragma once

#include "../Net/InPacket.h"

#include <cstdint>

namespace ms
{
	// The user-effect dispatcher (FUN_009377d9) serves the player's own
	// effects (0xCE, `mode, body`) and other users' (0xC6, `cid, mode, body`)
	// with one body per mode.
	namespace user_effect
	{
		// Read and show the modes that do the same thing on the player and on
		// anyone else, for the user `cid`. False leaves the mode, unread, to
		// the caller.
		bool show_shared(int32_t cid, int8_t mode, InPacket& recv);
	}
}
