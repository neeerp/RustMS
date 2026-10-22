#pragma once

#include "../PacketHandler.h"

namespace ms
{
	// The guild bulletin board's list, a thread, or a thread gone; dropped
	// unless the board is open
	// Opcode: GUILD_BBS(59)
	class GuildBbsHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};

	// The player's guild, founding one, and guild errors
	// Opcode: GUILD_OPERATION(65)
	class GuildOperationHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};

	// The guild's alliance, an invitation to one, and its members logging in
	// Opcode: ALLIANCE_OPERATION(66)
	class AllianceOperationHandler : public PacketHandler
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

	// "<Guild> Name is now married. Please congratulate them.", or a
	// family's
	// Opcode: NOTIFY_MARRIAGE(106)
	class NotifyMarriageHandler : public PacketHandler
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
