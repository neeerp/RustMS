#include "../Character/QuestRecordEx.h"
#include <iostream>
#include <map>
#include <stdexcept>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

int main()
{
	try
	{
		using ms::QuestRecordEx::token;
		using ms::QuestRecordEx::value;

		std::string raw = "try=12;have=1;rank=D;cmp=3;min=7;sec=5;date=2009/02/13";
		require(value(raw, "try") == "12", "a value");
		require(value(raw, "date") == "2009/02/13", "the last value");
		require(value(raw, "vic").empty(), "a missing key");
		require(value("", "try").empty(), "an empty record");

		auto get = [&](const std::string& key) { return value(raw, key); };
		require(token("try", get) == "  12", "counts are %4d");
		require(token("sec", get) == " 5", "seconds are %2d");
		require(token("vic", get) == "   0", "a missing count is 0");
		require(token("rank", get) == "#fUI/Basic.img/PQuestRank/D#", "the rank's image");
		require(token("have", get) == "Acquire", "a reward had");
		require(token("have0", get) == "Fail", "a reward not had");
		require(token("date", get) == "2009/02/13", "anything else as it stands");

		auto none = [](const std::string&) { return std::string(); };
		require(token("rank", none) == "#fUI/Basic.img/PQuestRank/F#", "no rank is F");

		std::string carnival = "try=5;vic=1;lose=2;draw=1";
		require(token("gvup", [&](const std::string& key) { return value(carnival, key); }) == "1",
				"give-ups are the tries less the rest");

		std::cout << "Quest record ex passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
