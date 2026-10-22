#pragma once

#include "../PacketHandler.h"

namespace ms
{
	// A player on the map proposes to this one
	// Opcode: MARRIAGE_REQUEST(72)
	class MarriageRequestHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};

	// A proposal's end: the couple engaged, or why not
	// Opcode: MARRIAGE_RESULT(73)
	class MarriageResultHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};
}
