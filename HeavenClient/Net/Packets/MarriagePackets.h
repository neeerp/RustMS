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
}
