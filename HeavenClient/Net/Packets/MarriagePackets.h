#pragma once

#include "../OutPacket.h"

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
}
