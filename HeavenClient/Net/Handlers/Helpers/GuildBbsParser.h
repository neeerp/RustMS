#pragma once

#include "../../InPacket.h"

#include "../../../Character/GuildBbs.h"

namespace ms
{
	namespace GuildBbsParser
	{
		// GUILD_BBS mode 6 (0x816c6f): a notice flag and the notice, the
		// count of the other threads, and, only if that is above 0, a page
		// of them.
		GuildBbs::List parse_list(InPacket& recv);

		// GUILD_BBS mode 7 (0x816fb7): the thread and its replies.
		GuildBbs::Thread parse_thread(InPacket& recv);
	}
}
