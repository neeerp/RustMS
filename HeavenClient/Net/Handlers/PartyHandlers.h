#pragma once

#include "../PacketHandler.h"

namespace ms
{
	// Party invitations, membership changes, and results
	// Opcode: PARTY_OPERATION(62)
	class PartyOperationHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};

	// HP of a party member on the player's map
	// Opcode: UPDATE_PARTYMEMBER_HP(201)
	class UpdatePartyMemberHpHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};

	// Buddy, party, or guild chat; only party chat is used
	// Opcode: MULTICHAT(134)
	class MultiChatHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};
}
