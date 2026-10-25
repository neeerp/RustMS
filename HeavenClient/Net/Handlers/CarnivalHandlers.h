#pragma once

#include "../PacketHandler.h"

namespace ms
{
	// The Monster Carnival's packets (CField_MonsterCarnival::OnPacket,
	// v83ref b163). The window that spends CP comes later; for now the
	// client keeps the sides and the CP, and writes the chat log's lines.

	// The battle begins, or the player enters the revive room: their side,
	// and on the battle field the CP and the mobs out
	// Opcode: MONSTER_CARNIVAL_START(289)
	class CarnivalStartHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};

	// The player's own CP
	// Opcode: MONSTER_CARNIVAL_OBTAINED_CP(290)
	class CarnivalObtainedCpHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};

	// A side's CP
	// Opcode: MONSTER_CARNIVAL_PARTY_CP(291)
	class CarnivalPartyCpHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};

	// Someone spent CP; answers the player's own spending too
	// Opcode: MONSTER_CARNIVAL_SUMMON(292)
	class CarnivalSummonHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};

	// Why the player's spending failed
	// Opcode: MONSTER_CARNIVAL_MESSAGE(293)
	class CarnivalMessageHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};

	// Someone died, and what their side lost
	// Opcode: MONSTER_CARNIVAL_DIED(294)
	class CarnivalDiedHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};

	// Someone left the battle
	// Opcode: MONSTER_CARNIVAL_LEAVE(295)
	class CarnivalLeaveHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};

	// How the battle ended
	// Opcode: MONSTER_CARNIVAL_RESULT(296)
	class CarnivalResultHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};
}
