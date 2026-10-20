#pragma once

#include "../PacketHandler.h"

namespace ms
{
	// Notes waiting for the player, and what became of one they sent
	// (v83 a2508b, report 204m).
	// Opcode: MEMO_RESULT(41)
	class MemoResultHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};
}
