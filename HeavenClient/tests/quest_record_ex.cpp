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

		using ms::QuestRecordEx::RankS;
		require(token("scnt", none, RankS { 9, 5 }) == "9", "the S count, unclamped");
		require(token("perPqS", none, RankS { 0, 5 }) == "0", "no S ranks is 0%");
		require(token("perPqS", none, RankS { 3, 5 }) == "60", "three of five is 60%");
		require(token("perPqS", none, RankS { 9, 5 }) == "100", "more than the need is 100%");
		require(token("perPqS", none, RankS { 3, 0 }) == "0", "no need is 0%");
		require(token("gaugePqS", none, RankS { 3, 5 }) == "#fUI/UIWindow.img/Quest/Gauge2/frame/45#", "the gauge's slices");
		require(token("gaugePqS", none, RankS { 9, 5 }) == "#fUI/UIWindow.img/Quest/Gauge2/frame/75#", "a full gauge");

		using ms::QuestRecordEx::gauge_slices;
		require(gauge_slices("UI/UIWindow.img/Quest/Gauge2/frame/45") == 45, "a gauge's path");
		require(gauge_slices("UI/UIWindow.img/Quest/Gauge2/frame/0") == 0, "an empty gauge");
		require(gauge_slices("UI/UIWindow.img/Quest/Gauge2/frame/") == -1, "no slices");
		require(gauge_slices("UI/Basic.img/PQuestRank/S") == -1, "another image");

		std::cout << "Quest record ex passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
