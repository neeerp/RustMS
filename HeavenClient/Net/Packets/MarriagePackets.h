#pragma once

#include "../OutPacket.h"

#include <string>
#include <vector>

namespace ms
{
	// Proposes to `name` with a ring box (RING_ACTION 0, report b052).
	class ProposePacket : public OutPacket
	{
	public:
		ProposePacket(const std::string& name, int32_t item_id) : OutPacket(OutPacket::Opcode::RING_ACTION)
		{
			write_byte(0);
			write_string(name);
			write_int(item_id);
		}
	};

	// Withdraws the player's proposal (RING_ACTION 1).
	class CancelProposalPacket : public OutPacket
	{
	public:
		CancelProposalPacket() : OutPacket(OutPacket::Opcode::RING_ACTION)
		{
			write_byte(1);
		}
	};

	// Answers `name`'s proposal (RING_ACTION 2).
	class AnswerProposalPacket : public OutPacket
	{
	public:
		AnswerProposalPacket(bool accept, const std::string& name, int32_t proposer) : OutPacket(OutPacket::Opcode::RING_ACTION)
		{
			write_byte(2);
			write_byte(accept);
			write_string(name);
			write_int(proposer);
		}
	};

	// Breaks off by dropping the token `item_id` (RING_ACTION 3, report
	// b054); the token stays until the server takes it.
	class BreakOffPacket : public OutPacket
	{
	public:
		BreakOffPacket(int32_t item_id) : OutPacket(OutPacket::Opcode::RING_ACTION)
		{
			write_byte(3);
			write_int(item_id);
		}
	};

	// Sends the invitation in Etc `slot` to `guest`, for the couple
	// `marriage_id` (RING_ACTION 5, report b069).
	class InviteGuestPacket : public OutPacket
	{
	public:
		InviteGuestPacket(const std::string& guest, int32_t marriage_id, int16_t slot) : OutPacket(OutPacket::Opcode::RING_ACTION)
		{
			write_byte(5);
			write_string(guest);
			write_int(marriage_id);
			write_int(slot);
		}
	};

	// Opens the received invitation `item_id` in Etc `slot` (RING_ACTION 6).
	class OpenInvitationPacket : public OutPacket
	{
	public:
		OpenInvitationPacket(int16_t slot, int32_t item_id) : OutPacket(OutPacket::Opcode::RING_ACTION)
		{
			write_byte(6);
			write_int(slot);
			write_int(item_id);
		}
	};

	// Hands in the wishlist the wedding staff asked for (RING_ACTION 9,
	// report b084).
	class WishlistPacket : public OutPacket
	{
	public:
		WishlistPacket(const std::vector<std::string>& wishes) : OutPacket(OutPacket::Opcode::RING_ACTION)
		{
			write_byte(9);
			write_short(static_cast<int16_t>(wishes.size()));

			for (const std::string& wish : wishes)
				write_string(wish);
		}
	};

	// What the Wedding Registry and the gift window ask (WEDDING_ACTION):
	// give `qty` of the item in `slot` of its bag, take a gift, or say the
	// window closed.
	class WeddingActionPacket : public OutPacket
	{
	public:
		WeddingActionPacket(int16_t slot, int32_t item_id, int16_t qty) : OutPacket(OutPacket::Opcode::WEDDING_ACTION)
		{
			write_byte(6);
			write_short(slot);
			write_int(item_id);
			write_short(qty);
		}

		// The gift at `index` of its type's list, the type being the
		// inventory's (1 equip to 5 cash).
		WeddingActionPacket(uint8_t type, uint8_t index) : OutPacket(OutPacket::Opcode::WEDDING_ACTION)
		{
			write_byte(7);
			write_byte(type);
			write_byte(index);
		}

		WeddingActionPacket() : OutPacket(OutPacket::Opcode::WEDDING_ACTION)
		{
			write_byte(8);
		}
	};

	// The couple's client is done with the priest's page; `step` is the
	// latest one it heard (0x8B, report b075).
	class WeddingTalkPacket : public OutPacket
	{
	public:
		WeddingTalkPacket(uint8_t step) : OutPacket(OutPacket::Opcode::WEDDING_TALK)
		{
			write_byte(step);
		}
	};

	// A Cathedral guest blesses the couple (0x8C, no body).
	class BlessCouplePacket : public OutPacket
	{
	public:
		BlessCouplePacket() : OutPacket(OutPacket::Opcode::WEDDING_BLESS) {}
	};
}
