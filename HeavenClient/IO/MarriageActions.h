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
	}
}
