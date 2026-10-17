#pragma once

#include <cstdint>
#include <string>
#include <vector>

namespace ms
{
	// How long a skill's body action keeps the user from acting, in ms, as
	// USE_SKILL ends with it (0x96d425). The client builds it from
	// 00002000.img as it starts (FUN_00406abd).
	namespace action_lock
	{
		// Whether the action plays back and forth, which locks nothing.
		inline bool back_and_forth(const std::string &action)
		{
			return action == "stand1" || action == "stand2" || action == "alert" || action == "ghoststand";
		}

		// An alias action locks for its frames with a negative delay.
		inline uint16_t alias(const std::vector<int16_t> &delays)
		{
			int32_t lock = 0;

			for (int16_t delay : delays)
				if (delay < 0)
					lock -= delay;

			return static_cast<uint16_t>(lock);
		}

		// One of the body's own actions locks for all but its last frame.
		inline uint16_t direct(const std::string &action, const std::vector<uint16_t> &delays)
		{
			if (delays.empty() || back_and_forth(action))
				return 0;

			int32_t lock = 0;

			for (size_t i = 0; i + 1 < delays.size(); i++)
				lock += delays[i];

			return static_cast<uint16_t>(lock);
		}
	}
}
