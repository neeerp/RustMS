#pragma once

#include "../../InPacket.h"

#include "../../../Character/Alliance.h"
#include "../../../Character/Guild.h"

namespace ms
{
	namespace GuildParser
	{
		// GUILDDATA (FUN_004e4553): the guild, its titles, and its members,
		// each as a fixed 37-byte record.
		GuildData parse_guild(InPacket& recv);

		// One GUILDMEMBER record, which leaves out the id.
		GuildMember parse_member(InPacket& recv);

		// The emblem as GUILDDATA, SPAWN_PLAYER, and GUILD_MARK_CHANGED
		// carry it.
		GuildEmblem parse_emblem(InPacket& recv);

		// The ALLIANCE struct (FUN_004e475a), the guild ids before the
		// capacity.
		AllianceData parse_alliance(InPacket& recv);
	}
}
