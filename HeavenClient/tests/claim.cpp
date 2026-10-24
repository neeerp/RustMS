#include "../Character/Claim.h"
#include <iostream>
#include <stdexcept>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

int main()
{
	try
	{
		using namespace ms::Claim;
		using Names = std::vector<std::string>;

		require(in_hours(0, 0, 0) && in_hours(0, 0, 23), "0 and 0 is all day");
		require(in_hours(9, 17, 9) && in_hours(9, 17, 16), "from the start hour up to the end");
		require(!in_hours(9, 17, 17) && !in_hours(9, 17, 8), "not the end hour, nor before the start");
		require(in_hours(22, 6, 23) && in_hours(22, 6, 0) && in_hours(22, 6, 5), "past midnight");
		require(!in_hours(22, 6, 6) && !in_hours(22, 6, 21), "past midnight, not outside");
		require(!in_hours(5, 5, 5) && !in_hours(5, 5, 6), "the same hours, never");

		const std::string down = "The reporting system is currently undergoing maintenence check.";
		require(!refusal(true, 0, 0, 12) && !refusal(true, 9, 17, 9), "open, in the hours");
		require(*refusal(false, 0, 0, 12) == down, "closed, whatever the hours");
		require(*refusal(true, 5, 5, 5) == down, "the same hours, not 0 and 0, count as down");
		require(*refusal(true, 9, 17, 17) == "You may only report from 9 to 17.", "outside the hours");

		require(reasons(PROGRAM) == Names({ "Report Use of Illegal Programs" }), "a program's one reason");
		require(reasons(CONVERSATION).size() == 6 && reasons(CONVERSATION)[1] == "Advertising", "a conversation's six");
		require(reason_code(PROGRAM, 0) == 9, "a program sends 9");
		require(reason_code(CONVERSATION, 0) == 1 && reason_code(CONVERSATION, 2) == 3 &&
				reason_code(CONVERSATION, 3) == 5 && reason_code(CONVERSATION, 5) == 8,
				"a conversation's codes skip 4 and 6");

		require(speaker("Bob : hi") == "Bob", "the name before the colon");
		require(speaker("[Guild] Bob : hi : there") == "Bob", "the last word before the first colon");
		require(speaker("and the rest of a long line").empty(), "a wrapped row has no speaker");

		Names rows = { "Bob : hi", "Self : hello", "Carl : yo", "Bob : again", "a wrapped row" };
		require(speakers(rows, "Self") == Names({ "Bob", "Carl" }), "latest first, once each, not yourself");

		auto stars = [](const std::string& line) { return line + "*"; };
		require(chat_log(rows, "Bob", "Self", stars) == "Bob : hi\nSelf : hello*\nBob : again\n",
				"the two's lines, oldest first, yours filtered");
		require(chat_log(rows, "Nobody", "Self", stars) == "Self : hello*\n", "only yours if they never spoke");

		Names many;

		for (int i = 0; i < 64; i++)
			many.push_back("Bob : " + std::string(40, 'a' + i % 26));

		std::string log = chat_log(many, "Bob", "Self", stars);
		require(log.size() <= MAX_LOG && log.size() > MAX_LOG - 47, "1,600 characters at most");
		require(log.substr(log.size() - 47) == many.back() + "\n", "the latest lines kept");

		// Lines of 99 and a newline: 16 fit in 1,600 exactly, a 17th would not, and an older short one
		// isn't reached for.
		Names full = { "Bob : x" };

		for (int i = 0; i < 17; i++)
			full.push_back("Bob : " + std::string(93, 'a' + i));

		std::string capped = chat_log(full, "Bob", "Self", stars);
		require(capped.size() == 1600, "up to 1,600 with the newlines");
		require(capped.substr(0, 99) == full[2] && capped.find("Bob : x\n") == std::string::npos,
				"it stops at the first that doesn't fit");

		require(sorted({ "bobby", "Zed9", "Alice" }) == Names({ "Alice", "Zed9", "bobby" }), "as strcmp");

		require(description({ "  he ", "", "   ", " cheats" }) == "hecheats", "trimmed and run together");
		require(description({ "", " ", "", "" }).empty(), "nothing but spaces");

		require(*result_message(2, true, 9) ==
				"Your report has been successfully registered.\\r\\nYou have 9 reports left this week.",
				"the reports left");
		require(*result_message(2, true, 0) ==
				"Your report has been successfully registered.\\r\\nYou have no report left this week.",
				"none left");
		require(result_message(2, false, 3)->find("rest of the day") != std::string::npos, "none today");
		require(*result_message(3, false, 0) == "You have been reported by a user.", "reported");
		require(*result_message(0x41, false, 0) == "Please try again later.", "later");
		require(*result_message(0x42, false, 0) == "Please re-check the character name, then try again.", "the name");
		require(*result_message(0x43, false, 0) == "You do not have enough mesos to report.", "the fee");
		require(*result_message(0x44, false, 0) == "Unable to connect to the server.", "no server");
		require(*result_message(0x45, false, 0) == "You have exceeded\\r\\nthe number of reports available.",
				"the limit");
		require(result_message(0x48, false, 0)->find("false report") != std::string::npos, "a false report");
		start_hour = 9;
		end_hour = 17;
		require(*result_message(0x47, false, 0) == "You may only report from 9 to 17.", "the hours");
		require(!result_message(0x40, false, 0) && !result_message(0x46, false, 0), "no box for other modes");
	}
	catch (const std::exception& e)
	{
		std::cerr << "FAILED: " << e.what() << std::endl;
		return 1;
	}

	std::cout << "claim tests passed" << std::endl;
	return 0;
}
