#pragma once

#include <array>
#include <cstdint>
#include <optional>
#include <string>
#include <vector>

// The Lie Detector's client side (report c025): what ANTI_MACRO_RESULT
// shows, and whether a test is running.
namespace ms::lie_detector
{
	// The GM's Lie Detector skill.
	constexpr int32_t SKILL = 9001009;

	// ANTI_MACRO_RESULT's codes: what gave the test.
	constexpr uint8_t BY_ITEM = 1;
	constexpr uint8_t BY_SKILL = 2;

	// A first picture's time to answer, in milliseconds (FUN_00a265c2).
	constexpr int32_t TEST_TIME = 60000;
	// The clock shows the time left only up to this many seconds (7e778c).
	constexpr int32_t CLOCK_LIMIT = 120;

	// StringPool 0xc64, 0xc65 and 0xc5f.
	constexpr const char *IN_TEST = "You cannot use it in the middle of the Lie Detector Test.";
	constexpr const char *NOT_HERE = "You may not use the Lie Detector on this area.";
	constexpr const char *USED_BY = " have used the Lie Detector Test.";

	// ANTI_MACRO_RESULT's types.
	enum Type : uint8_t
	{
		NOT_FOUND = 0,
		NOT_ATTACKING = 1,
		TESTED = 2,
		IN_A_TEST = 3,
		MONITORED = 4,
		USED = 5,
		CHALLENGE = 6,
		FAILED = 7,
		SUSPECTED = 8,
		PASSED = 9,
		CLEARED = 10,
		FINE_RECEIVED = 11
	};

	// The types that carry a name.
	constexpr bool named(uint8_t type)
	{
		return type == MONITORED || type == USED || type == SUSPECTED || type == CLEARED;
	}

	// A line in chat, and whether a screenshot goes with it.
	struct Line
	{
		std::string text;
		bool screenshot;
	};

	// The chat line for a named type and `code` (FUN_00a265c2), with
	// `name`; nothing for a code it doesn't show. The `name`_ is the shot's
	// file name, which starts with it.
	inline std::optional<Line> line(uint8_t type, uint8_t code, const std::string &name)
	{
		switch (type)
		{
			case MONITORED:
				// StringPool 0xc60.
				return Line{ name + "_The screenshot has been saved. You have been notified of macro-assisted "
									"program monitoring.", true };
			case USED:
				if (code == BY_ITEM)
					return Line{ name + USED_BY, false };

				// StringPool 0xc61.
				if (code == BY_SKILL)
					return Line{ name + "_The screenshot has been saved. The Lie Detector has been activated.", true };

				return std::nullopt;
			case SUSPECTED:
				// StringPool 0xc63.
				if (code == BY_SKILL)
					return Line{ name + "_The screenshot has been saved. It appears that you may be using a "
										"macro-assisted program.", true };

				return std::nullopt;
			case CLEARED:
				// StringPool 0xc62.
				if (code == BY_SKILL)
					return Line{ name + "_You have passed the Lie Detector Test.", false };

				return std::nullopt;
			default:
				return std::nullopt;
		}
	}

	// What the result dialog shows: the GM's face, Macro/admin/N, and its
	// paragraphs.
	struct Result
	{
		int portrait;
		std::vector<std::string> lines;
	};

	// The result dialog for `type` and `code` (FUN_007e8a2f); nothing for
	// a type it doesn't show.
	inline std::optional<Result> result(uint8_t type, uint8_t code)
	{
		switch (type)
		{
			case NOT_FOUND:
				return Result{ 1, { "The user cannot be found." } };
			case NOT_ATTACKING:
				return Result{ 1, { "You cannot use it on a user that isn't in the middle of attack." } };
			case TESTED:
				return Result{ 1, { "This user has already been tested before." } };
			case IN_A_TEST:
				return Result{ 1, { "This user is currently going through the Lie Detector Test." } };
			case FAILED:
			{
				Result failed{ 2, { "The Lie Detector Test confirms that you have been botting. Repeated failure of "
									"the test will result in game restrictions." } };

				if (code == BY_SKILL)
					failed.lines.emplace_back("You will be sanctioned for using a macro-assisted program.");

				return failed;
			}
			case PASSED:
				if (code == 3)
					return Result{ 0, { "You have succesfully passed the Lie Detector Test. Thank you for "
										"participating!" } };

				if (code == BY_SKILL)
					return Result{ 0, { "Thank you for your cooperation." } };

				return Result{ 0, { "Thank you for cooperating with the Lie Detector Test. You'll be rewarded "
									"5000 mesos for not botting." } };
			case FINE_RECEIVED:
				return Result{ 0, { "The user has failed the Lie Detector Test. You'll be rewarded 7000 mesos "
									"from the user." } };
			default:
				return std::nullopt;
		}
	}

	// The clock's Macro/num1 digits for `ms` left, minutes then seconds,
	// two of each; nothing outside 0 to CLOCK_LIMIT seconds.
	inline std::optional<std::array<uint8_t, 4>> clock(int32_t ms)
	{
		int32_t seconds = ms / 1000;

		if (ms < 0 || seconds > CLOCK_LIMIT)
			return std::nullopt;

		int32_t minutes = seconds / 60;
		seconds %= 60;

		return std::array<uint8_t, 4>{
				static_cast<uint8_t>(minutes / 10), static_cast<uint8_t>(minutes % 10),
				static_cast<uint8_t>(seconds / 10), static_cast<uint8_t>(seconds % 10)
		};
	}

	// The tries left a picture shows (0x7e6a94): two with the first, one
	// with the next.
	constexpr int32_t tries(bool first)
	{
		return first ? 2 : 1;
	}

	// The time left to answer while a test runs, else 0 (ctx+0x36fc): set
	// by a first picture, kept from one picture to the next and cleared by
	// the result or a new field.
	inline int32_t time_left = 0;

	inline bool testing()
	{
		return time_left != 0;
	}

	// The time for a picture: a first one starts the clock, the next one
	// takes the time left.
	inline int32_t picture_time(bool first)
	{
		if (first)
			time_left = TEST_TIME;

		return time_left;
	}
}
