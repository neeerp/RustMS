#pragma once

#include "../../InPacket.h"

#include "../../../Character/Alliance.h"
#include "../../../Character/Guild.h"

#include <cstdint>
#include <string>

namespace ms
{
	// ALLIANCE_OPERATION(66)'s modes but the invitation, applied to the
	// player's alliance and guild as FUN_00a39f4e does, apart from what the
	// handler shows and sends.
	namespace AllianceNews
	{
		enum Mode : uint8_t
		{
			INVITE = 0x03,
			ALLIANCE = 0x0C,
			GUILDS = 0x0D,
			MEMBER_ONLINE = 0x0E,
			INFO = 0x0F,
			GUILD_LEFT = 0x10,
			GUILD_JOINED = 0x12,
			CHANGED = 0x17,
			MEMBER_LEVEL_JOB = 0x18,
			MASTER = 0x19,
			TITLES = 0x1A,
			RANK = 0x1B,
			NOTICE = 0x1C,
			DISBANDED = 0x1D,
			GUILD_CHANGED = 0x1E
		};

		struct Outcome
		{
			// A chat line of type 0xc, or "".
			std::string chat;
			// Someone in another guild logged in, for the card; or "".
			std::string logged_in;
			// The player's guild joined: ask for the alliance (0x8F 1).
			bool ask_info = false;
		};

		// `self` is the player's character id. Unknown modes read nothing.
		Outcome apply(uint8_t mode, InPacket& recv, Alliance& alliance, Guild& own, int32_t self);
	}
}
