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

	// Where the partner is now: only kept, for the minimap and the world
	// map to show (0xa0831d, report b057)
	// Opcode: NOTIFY_MARRIED_PARTNER_MAP_TRANSFER(75)
	class PartnerMapHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};
}
