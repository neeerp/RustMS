#pragma once

#include "../PacketHandler.h"

namespace ms
{
	// Coconut Season's packets (CField_Coconut, v83ref b167). Its clock is
	// the CLOCK handler's.

	// The player has been moved to `mapid`: whether it is a coconut field,
	// its count of nuts and its uniforms, and off with the last one's.
	void enter_coconut_field(int32_t mapid, int32_t fieldtype);

	// What a nut, or all of them, do after a delay
	// Opcode: COCONUT_HIT(285)
	class CoconutHitHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};

	// The two teams' scores
	// Opcode: COCONUT_SCORE(286)
	class CoconutScoreHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};

	// The player's team, whose uniform they wear on the field
	// Opcode: FIELD_SPECIFIC_DATA(133)
	class FieldSpecificDataHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};
}
