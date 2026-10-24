#pragma once

#include <algorithm>
#include <cstdint>
#include <functional>
#include <optional>
#include <string>
#include <vector>

// Reporting a player to a GM (v83 CLAIM, reports b155-report-user, b155-s2-client): CWvsContext::SendClaim
// (0xa2719c) asks for the kind of report, then for the character, the reason and a description, and
// sends them with the lines of the chat log the two spoke.
namespace ms::Claim
{
	// What the server said: whether reports are taken (CLAIM_STATUS_CHANGED) and in which hours of the
	// day (CLAIM_AVAILABLE_TIME). Until it says, the reporting system is down.
	inline bool open = false;
	inline uint8_t start_hour = 0;
	inline uint8_t end_hour = 0;

	// The notice's buttons: a conversation (0x3e8) or an illegal program (0x3e9).
	enum Kind : uint8_t
	{
		PROGRAM,
		CONVERSATION
	};

	// The most of the chat log a report carries (0x490b94).
	constexpr size_t MAX_LOG = 1600;
	// Each of the description's four lines (0x7db47b).
	constexpr size_t LINE_LIMIT = 24;

	// Whether a report may be made at `hour` (local time) between `start` and `end`; 0 and 0 is all day,
	// and a window may run past midnight (0xa271d7).
	inline bool in_hours(uint8_t start, uint8_t end, int hour)
	{
		if (start == 0 && end == 0)
			return true;

		if (start < end)
			return start <= hour && hour < end;

		if (end < start)
			return start <= hour || hour < end;

		return false;
	}

	// Why a report can't be started now, at `hour` (local time), as SendClaim says it (0xa2719c): the
	// system is down, or it is outside the hours. Hours the same but not 0 and 0 count as down.
	inline std::optional<std::string> refusal(bool open, uint8_t start, uint8_t end, int hour)
	{
		if (in_hours(start, end, hour) && open)
			return std::nullopt;

		if (!open || start == end)
			return "The reporting system is currently undergoing maintenence check.";

		return "You may only report from " + std::to_string(start) + " to " + std::to_string(end) + ".";
	}

	// The reasons the second window offers, in order (0x7db17d): a conversation's six, or the one
	// for a program.
	inline std::vector<std::string> reasons(Kind kind)
	{
		if (kind == PROGRAM)
			return { "Report Use of Illegal Programs" };

		return { "Curse/Inappropriate Content", "Advertising", "Fraud", "Cash Trade", "Impersonating GM",
				 "Exposing Personal Info" };
	}

	// The reason sent for the one picked (0x7dbaa3).
	inline uint8_t reason_code(Kind kind, uint16_t index)
	{
		if (kind == PROGRAM)
			return 9;

		constexpr uint8_t CODES[] = { 1, 2, 3, 5, 7, 8 };

		return index < 6 ? CODES[index] : 0;
	}

	// Who spoke a chat row, "Name : text": the last word before the first " : " (0x490e93). A row
	// that isn't a line someone spoke, like a wrapped line's later rows, has no speaker.
	inline std::string speaker(const std::string& row)
	{
		size_t colon = row.find(" : ");

		if (colon == std::string::npos)
			return "";

		std::string before = row.substr(0, colon);
		size_t space = before.find_last_of(' ');

		return space == std::string::npos ? before : before.substr(space + 1);
	}

	// Whose lines a conversation report can be about: everyone who spoke in `rows` (oldest first) but
	// `self`, the latest speaker first (0x490927).
	inline std::vector<std::string> speakers(const std::vector<std::string>& rows, const std::string& self)
	{
		std::vector<std::string> names;

		for (auto row = rows.rbegin(); row != rows.rend(); ++row)
		{
			std::string name = speaker(*row);

			if (!name.empty() && name != self && std::find(names.begin(), names.end(), name) == names.end())
				names.push_back(name);
		}

		return names;
	}

	// The chat log a report sends: the latest of the lines `victim` or `self` spoke in `rows`, up to
	// 1,600 characters with a newline each, oldest first (0x490b94). The reporter's own lines pass
	// through `filter`, as v83 stars out their curse words.
	inline std::string chat_log(const std::vector<std::string>& rows, const std::string& victim,
								const std::string& self, const std::function<std::string(const std::string&)>& filter)
	{
		std::string log;
		size_t total = 0;

		for (auto row = rows.rbegin(); row != rows.rend(); ++row)
		{
			std::string name = speaker(*row);

			if (name.empty() || (name != victim && name != self))
				continue;

			std::string line = name == self ? filter(*row) : *row;
			total += line.size() + 1;

			if (total > MAX_LOG)
				break;

			log = line + "\n" + log;
		}

		return log;
	}

	// The characters a program report may name: those on the map, sorted as strcmp does (0xa2719c).
	inline std::vector<std::string> sorted(std::vector<std::string> names)
	{
		std::sort(names.begin(), names.end());

		return names;
	}

	// The description: the four lines, each trimmed, run together (0x7db8df).
	inline std::string description(const std::vector<std::string>& lines)
	{
		std::string text;

		for (const auto& line : lines)
		{
			size_t first = line.find_first_not_of(' ');

			if (first != std::string::npos)
				text += line.substr(first, line.find_last_not_of(' ') - first + 1);
		}

		return text;
	}

	// The OK box CLAIM_RESULT shows (0xa27891): `today` and `left` come with mode 2.
	inline std::optional<std::string> result_message(uint8_t mode, bool today, int32_t left)
	{
		switch (mode)
		{
			case 2:
				if (!today)
					return "The report has been successfully made.\\r\\nYou may not report for the rest of the day.";

				if (left > 0)
					return "Your report has been successfully registered.\\r\\nYou have " + std::to_string(left) +
						   " reports left this week.";

				return "Your report has been successfully registered.\\r\\nYou have no report left this week.";
			case 3:
				return "You have been reported by a user.";
			case 0x41:
				return "Please try again later.";
			case 0x42:
				return "Please re-check the character name, then try again.";
			case 0x43:
				return "You do not have enough mesos to report.";
			case 0x44:
				return "Unable to connect to the server.";
			case 0x45:
				return "You have exceeded\\r\\nthe number of reports available.";
			case 0x47:
				return "You may only report from " + std::to_string(start_hour) + " to " + std::to_string(end_hour) +
					   ".";
			case 0x48:
				return "Unable to report due to\\n\\npreviously being cited for a false report.";
			default:
				return std::nullopt;
		}
	}
}
