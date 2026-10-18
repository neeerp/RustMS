#pragma once

#include <cstdint>
#include <string>

namespace ms
{
	// The room a character keeps open, as the balloon over them shows it
	// (UPDATE_CHAR_BOX and the player spawn, v83 FUN_0048bbca).
	struct MiniRoomBox
	{
		int8_t type = 0;
		int32_t serial = 0;
		std::string title;
		uint8_t skin = 0;
		uint8_t users = 0;
		uint8_t max_users = 0;
	};
}
