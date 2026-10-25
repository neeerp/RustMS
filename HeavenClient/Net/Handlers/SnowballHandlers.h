#pragma once

#include "../PacketHandler.h"

namespace ms
{
	// Snowball's packets (CField_SnowBall, v83ref b168). Its clock is the
	// CLOCK handler's.

	// The player has been moved to `mapid`: whether it is a snowball field
	// and its lanes, and off with the last game.
	void enter_snowball_field(int32_t mapid, int32_t fieldtype);

	// The balls' steps and paces, and the snowmen's HP
	// Opcode: SNOWBALL_STATE(281)
	class SnowballStateHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};

	// What a hit on a ball or a snowman shows, after a delay
	// Opcode: HIT_SNOWBALL(282)
	class HitSnowballHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};

	// A line about a team's ball
	// Opcode: SNOWBALL_MESSAGE(283)
	class SnowballMessageHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};

	// The player walked into a ball, which throws them back
	// Opcode: LEFT_KNOCK_BACK(284)
	class LeftKnockBackHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};
}
