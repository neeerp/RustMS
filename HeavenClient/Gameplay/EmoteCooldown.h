#pragma once

#include <cstdint>

namespace ms
{
	// v83 lets the player make a face (F1-F7) at most once every 2 seconds
	// (FUN_00a24470); presses in between neither change the face nor reach
	// the server.
	class EmoteCooldown
	{
	public:
		// Whether an emote may start at now_ms; if so, it starts the wait.
		bool try_start(int64_t now_ms)
		{
			if (started && now_ms - last < DELAY)
				return false;

			started = true;
			last = now_ms;

			return true;
		}

	private:
		static constexpr int64_t DELAY = 2000;

		bool started = false;
		int64_t last = 0;
	};
}
