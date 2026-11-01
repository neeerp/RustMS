#pragma once

#include "../PacketHandler.h"

namespace ms
{
	// Ergoth's room's packets (CField_GuildBoss, v83ref b380).

	// The player has been moved to `mapid`: its healer and pulley if it is
	// Ergoth's room.
	void enter_guild_boss_field(int32_t mapid, int32_t fieldtype);

	// Where the chandelier hangs now
	// Opcode: GUILD_BOSS_HEALER_MOVE(287)
	class GuildBossHealerMoveHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};

	// Whether the pulley is locked
	// Opcode: GUILD_BOSS_PULLEY_STATE(288)
	class GuildBossPulleyStateHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};
}
