#pragma once

#include "../OutPacket.h"

#include "../../Template/Point.h"

namespace ms
{
	// Asks the server to start, complete, or forfeit a quest. The client shows
	// the quest's conversation itself (Quest.nx Say.img) and sends this when the
	// player accepts or finishes it.
	// Opcode: QUEST_ACTION(107)
	class QuestActionPacket : public OutPacket
	{
	public:
		enum Action : int8_t
		{
			START = 1,
			COMPLETE = 2,
			FORFEIT = 3,
			// The server runs the step as a script and drives the dialogue.
			SCRIPT_START = 4,
			SCRIPT_END = 5
		};

		QuestActionPacket(Action action, int16_t quest_id, int32_t npc_id, Point<int16_t> position) :
			OutPacket(OutPacket::Opcode::QUEST_ACTION)
		{
			write_byte(action);
			write_short(quest_id);

			if (action != FORFEIT)
			{
				write_int(npc_id);
				write_short(position.x());
				write_short(position.y());
			}
		}
	};
}
