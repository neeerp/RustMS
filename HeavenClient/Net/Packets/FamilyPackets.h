#pragma once

#include "../OutPacket.h"

namespace ms
{
	// Asks for a Family Tree, the player's own or another's
	// (OPEN_FAMILY_PEDIGREE).
	class OpenFamilyPedigreePacket : public OutPacket
	{
	public:
		OpenFamilyPedigreePacket(const std::string& name) : OutPacket(OutPacket::Opcode::OPEN_FAMILY_PEDIGREE)
		{
			write_string(name);
		}
	};

	// The Family window opened (OPEN_FAMILY): the server answers with the
	// player's numbers and the entitlements.
	class OpenFamilyPacket : public OutPacket
	{
	public:
		OpenFamilyPacket() : OutPacket(OutPacket::Opcode::OPEN_FAMILY) {}
	};

	// Asks a character on the player's map to be their junior
	// (ADD_FAMILY).
	class AddFamilyPacket : public OutPacket
	{
	public:
		AddFamilyPacket(const std::string& name) : OutPacket(OutPacket::Opcode::ADD_FAMILY)
		{
			write_string(name);
		}
	};

	// The answer to a senior's invitation (ACCEPT_FAMILY).
	class AcceptFamilyPacket : public OutPacket
	{
	public:
		AcceptFamilyPacket(int32_t inviter_id, const std::string& inviter, bool accept) : OutPacket(OutPacket::Opcode::ACCEPT_FAMILY)
		{
			write_int(inviter_id);
			write_string(inviter);
			write_byte(accept);
		}
	};

	// Uses the entitlement at `index` of the list, on a member for types 0
	// and 1 (USE_FAMILY).
	class UseFamilyPacket : public OutPacket
	{
	public:
		UseFamilyPacket(int32_t index) : OutPacket(OutPacket::Opcode::USE_FAMILY)
		{
			write_int(index);
		}

		UseFamilyPacket(int32_t index, const std::string& target) : UseFamilyPacket(index)
		{
			write_string(target);
		}
	};

	// New precepts from the leader (CHANGE_FAMILY_MESSAGE).
	class ChangeFamilyMessagePacket : public OutPacket
	{
	public:
		ChangeFamilyMessagePacket(const std::string& precepts) : OutPacket(OutPacket::Opcode::CHANGE_FAMILY_MESSAGE)
		{
			write_string(precepts);
		}
	};
}
