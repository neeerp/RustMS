#pragma once

#include "../PacketHandler.h"

namespace ms
{
	// The buddy list, requests to join it, buddies' channels, and errors
	// Opcode: BUDDYLIST(63)
	class BuddyListHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};
}
