#pragma once

#include "../PacketHandler.h"

namespace ms
{
	// Nett's Pyramid and the Dusty Platform (v83ref b160, b328). The stage's
	// counts come as SESSION_VALUEs and its clock as a CLOCK of kind 2.

	// How far kills have pushed the stage's gauge back: int count. Read only
	// on a stage (0x560a81)
	// Opcode: PYRAMID_GAUGE(157)
	class PyramidGaugeHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};

	// The run's result: byte rank (0 for S to 4 for D), int EXP. Read only
	// in the result room (0x5617c5)
	// Opcode: PYRAMID_SCORE(158)
	class PyramidScoreHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};
}
