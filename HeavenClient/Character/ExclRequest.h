#pragma once

#include <cstdint>

namespace ms
{
	// The v83 client lets one item, stat or skill request wait on the server
	// at a time (FUN_00485bf7). A request may go when none is waiting and
	// some milliseconds have passed since the last request or answer; the
	// answer is an inventory, stat or skill update with its first byte set,
	// or character info.
	class ExclRequest
	{
	public:
		// Milliseconds a pickup, an item use, meso drop, chair or character
		// info, a skill that doesn't attack, and anything else (moving,
		// gathering and sorting items, AP and SP) wait.
		static constexpr int32_t PICKUP_WAIT = 30;
		static constexpr int32_t USE_WAIT = 200;
		static constexpr int32_t SKILL_WAIT = 300;
		static constexpr int32_t WAIT = 500;

		bool can_send(int64_t now, int32_t wait) const
		{
			return !pending && now - last >= wait;
		}

		void sent(int64_t now)
		{
			pending = true;
			last = now;
		}

		void answered(int64_t now)
		{
			pending = false;
			last = now;
		}

		// Entering a field forgets a waiting request, but not its time.
		void clear()
		{
			pending = false;
		}

	private:
		bool pending = false;
		int64_t last = INT32_MIN;
	};
}
