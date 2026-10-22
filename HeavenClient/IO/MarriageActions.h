#pragma once

#include <cstdint>

namespace ms
{
	// The engagement a ring box starts (report b052).
	namespace MarriageActions
	{
		// A ring box used: the client's own refusals, the partner's name,
		// and the wait for her answer, which the player may withdraw.
		void propose(int32_t item_id);
		// MARRIAGE_RESULT, whatever it says, or a new field after a channel
		// change: the wait is over.
		void answered();
		// The couple's invitation in Etc `slot` double-clicked: ask whom to
		// send it to (0xa10199, report b069).
		void invite(int16_t slot);
		// A guest's invitation in Etc `slot` double-clicked: ask the server
		// for the window, unless the map forbids it (0xa103e7).
		void open_invitation(int16_t slot, int32_t item_id);
	}
}
