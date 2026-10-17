#pragma once

#include <cstdint>

namespace ms
{
	// v83 sends GIVE_FAME at most once every 5 minutes (FUN_00a23eb5); a
	// press in between says "Please try it again later." and sends nothing.
	class FameCooldown
	{
	public:
		// Whether fame may be given at now_ms; if so, it starts the wait.
		bool try_give(int64_t now_ms)
		{
			if (given && now_ms - last < DELAY)
				return false;

			given = true;
			last = now_ms;

			return true;
		}

	private:
		static constexpr int64_t DELAY = 300000;

		bool given = false;
		int64_t last = 0;
	};
}
