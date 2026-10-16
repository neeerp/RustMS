#pragma once

#include <array>
#include <cstdint>
#include <deque>
#include <limits>
#include <string>

namespace ms
{
	// v83's check before a chat line is sent (FUN_004904be). A line that is
	// the fourth identical one in a row, or the fourth within 2 seconds, is
	// refused with a warning; for 2.8 seconds after a warning, every line is
	// dropped silently. The run of identical lines starts over 30 seconds
	// after it began.
	class ChatFloodGuard
	{
	public:
		enum class Result
		{
			SEND,
			// Too soon after a warning: dropped without a word.
			MUTED,
			// "Repeating the same line over and over..."
			REPEATED,
			// "Too much chatting..."
			TOO_FAST
		};

		Result check(const std::string& line, int64_t now_ms)
		{
			if (now_ms - warned <= MUTE)
				return Result::MUTED;

			if (now_ms - history_start > HISTORY)
			{
				history_start = now_ms;
				history.clear();
			}

			history.push_back(line);

			if (history.size() > LINES)
				history.pop_front();

			if (history.size() == LINES && all_same())
			{
				warned = now_ms;
				return Result::REPEATED;
			}

			sent[next] = now_ms;
			next = (next + 1) % LINES;

			if (now_ms - sent[next] < BURST)
			{
				warned = now_ms;
				return Result::TOO_FAST;
			}

			return Result::SEND;
		}

	private:
		bool all_same() const
		{
			for (const auto& line : history)
				if (line != history.front())
					return false;

			return true;
		}

		static constexpr size_t LINES = 4;
		static constexpr int64_t MUTE = 2799, HISTORY = 30000, BURST = 2000;
		// Long before any chat, like the zeroed times v83 starts with.
		static constexpr int64_t NEVER = std::numeric_limits<int64_t>::min() / 2;

		int64_t warned = NEVER, history_start = NEVER;
		std::deque<std::string> history;
		// When the last four lines were sent; next is the oldest.
		std::array<int64_t, LINES> sent = { NEVER, NEVER, NEVER, NEVER };
		size_t next = 0;
	};
}
