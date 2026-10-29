#pragma once

#include "../PacketHandler.h"

namespace ms
{
	// The Ariant Coliseum's Battle Arena (v83ref ariant-s4-hud). Only the
	// arena field reads them.

	// Each player's Spirit Jewels, or who left the board
	// Opcode: ARIANT_ARENA_USER_SCORE(297)
	class AriantScoreHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};

	// The battle's results are in: the banner unrolls. No body
	// Opcode: ARIANT_RESULT(155)
	class AriantResultHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};
}
