#pragma once

#include "../PacketHandler.h"

namespace ms
{
	// The player's guild, founding one, and guild errors
	// Opcode: GUILD_OPERATION(65)
	class GuildOperationHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};

	// "<Guild> Name has reached Lv. 30.", or a family's
	// Opcode: NOTIFY_LEVELUP(105)
	class NotifyLevelUpHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};

	// "<Guild> Name has advanced to a(an) Fighter.", or a family's
	// Opcode: NOTIFY_JOB_CHANGE(107)
	class NotifyJobChangeHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};

	// The guild name under another character's name
	// Opcode: GUILD_NAME_CHANGED(202)
	class GuildNameChangedHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};

	// The guild emblem under another character's name
	// Opcode: GUILD_MARK_CHANGED(203)
	class GuildMarkChangedHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};
}
