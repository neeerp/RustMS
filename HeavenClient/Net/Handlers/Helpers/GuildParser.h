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

		// The ranking (FUN_00a38a98, mode 0x49) after the NPC's id: a count
		// and the guilds, each with its emblem as four ints, the mark first.
		std::vector<GuildStanding> parse_ranking(InPacket& recv);
	}
}
