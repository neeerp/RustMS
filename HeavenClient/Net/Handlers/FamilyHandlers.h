#pragma once

#include "../PacketHandler.h"

namespace ms
{
	// The Family Tree of the character asked for; dropped unless the tree
	// is open
	// Opcode: FAMILY_CHART_RESULT(94)
	class FamilyChartHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};

	// The player's family, juniors, Rep and entitlements used today
	// Opcode: FAMILY_INFO_RESULT(95)
	class FamilyInfoHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};

	// A refusal or the end of a relationship
	// Opcode: FAMILY_RESULT(96)
	class FamilyResultHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};

	// Someone asks the player to be their junior
	// Opcode: FAMILY_JOIN_REQUEST(97)
	class FamilyJoinRequestHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};

	// The one asked said yes or no
	// Opcode: FAMILY_JOIN_REQUEST_RESULT(98)
	class FamilyJoinRequestResultHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};

	// The player has a senior now
	// Opcode: FAMILY_JOIN_ACCEPTED(99)
	class FamilyJoinAcceptedHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};

	// The entitlements the Family window offers
	// Opcode: FAMILY_PRIVILEGE_LIST(100)
	class FamilyPrivilegeListHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};

	// Rep gained or spent
	// Opcode: FAMILY_REP_GAIN(101)
	class FamilyRepGainHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};

	// A relative logged in or out
	// Opcode: FAMILY_NOTIFY_LOGIN_OR_LOGOUT(102)
	class FamilyNotifyLoginOrLogoutHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};

	// The entitlement working for the player, or none
	// Opcode: FAMILY_SET_PRIVILEGE(103)
	class FamilySetPrivilegeHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};

	// A relative asks to summon the player
	// Opcode: FAMILY_SUMMON_REQUEST(104)
	class FamilySummonRequestHandler : public PacketHandler
	{
	public:
		void handle(InPacket& recv) const override;
	};
}
