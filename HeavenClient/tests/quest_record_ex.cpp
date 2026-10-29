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

		using ms::QuestRecordEx::Counts;
		using ms::QuestRecordEx::Tally;
		auto rank_s = [](int32_t count, int32_t need) { Counts counts; counts.rank_s = { count, need }; return counts; };
		require(token("scnt", none, rank_s(9, 5)) == "9", "the S count, unclamped");
		require(token("perPqS", none, rank_s(0, 5)) == "0", "no S ranks is 0%");
		require(token("perPqS", none, rank_s(3, 5)) == "60", "three of five is 60%");
		require(token("perPqS", none, rank_s(9, 5)) == "100", "more than the need is 100%");
		require(token("perPqS", none, rank_s(3, 0)) == "0", "no need is 0%");
		require(token("gaugePqS", none, rank_s(3, 5)) == "#fUI/UIWindow.img/Quest/Gauge2/frame/45#", "the gauge's slices");
		require(token("gaugePqS", none, rank_s(9, 5)) == "#fUI/UIWindow.img/Quest/Gauge2/frame/75#", "a full gauge");

		Counts specialist;
		specialist.completed = { 200, 800 };
		require(token("cmpcnt", none, specialist) == "200", "the quests completed");
		require(token("perQCnt", none, specialist) == "25", "200 of 800 is 25%");
		require(token("gaugeQCnt", none, specialist) == "#fUI/UIWindow.img/Quest/Gauge2/frame/18#", "75 slices' quarter, rounded down");
		require(token("scnt", none, specialist) == "0", "each title reads its own count");
		specialist.completed = { 900, 800 };
		require(token("perQCnt", none, specialist) == "100", "past the need is 100%");
		require(Tally { 5, -1 }.share(100) == 0, "no need is 0%");

		Counts stats;
		stats.level = 120;
		stats.fame = -3;
		require(token("level", none, stats) == "120" && token("pop", none, stats) == "-3", "level and fame as %d");

		// Outstanding Citizen: the fame gained since popS against popG.
		std::string citizen = "popS=10;popG=20";
		auto citizen_get = [&](const std::string& key) { return value(citizen, key); };
		Counts famous;
		famous.fame = 15;
		require(token("popgap", citizen_get, famous) == "5", "fame less popS");
		require(token("perPop", citizen_get, famous) == "25", "5 of 20 is 25%");
		require(token("gaugePop", citizen_get, famous) == "#fUI/UIWindow.img/Quest/Gauge2/frame/18#", "a quarter gauge");
		require(token("popG", citizen_get, famous) == "20", "popG as it stands");
		famous.fame = 5;
		require(token("popgap", citizen_get, famous) == "-5" && token("perPop", citizen_get, famous) == "0", "fame lost");
		famous.fame = 1;
		require(token("perPop", none, famous) == "100", "no popG divides by 1");
		require(token("popgap", none, famous) == "1", "no popS is plain fame");
		std::string zero = "popG=0;mg=0;mon=3";
		auto zero_get = [&](const std::string& key) { return value(zero, key); };
		require(token("perPop", zero_get, famous) == "0", "popG 0, where the client divides by zero");
		require(token("gaugePop", zero_get, famous) == "#fUI/UIWindow.img/Quest/Gauge2/frame/75#", "the gauge takes popG 0 as 1");

		// Veteran Hunter: mon against mg.
		std::string hunter = "mon=150;mg=600";
		auto hunter_get = [&](const std::string& key) { return value(hunter, key); };
		require(token("perHunt", hunter_get) == "25" && token("mon", hunter_get) == "150", "150 of 600");
		require(token("gaugeHunt", hunter_get) == "#fUI/UIWindow.img/Quest/Gauge2/frame/18#", "a quarter gauge");
		require(token("perHunt", zero_get) == "100", "mg 0 counts as 1");

		// Diligent Explorer: CNT days of 27 (30 days' nine tenths).
		Counts month;
		month.time_limit = 2592000;
		std::string diligent = "LT=2009/02/13;DONE=1;PT=10800;CNT=9";
		auto diligent_get = [&](const std::string& key) { return value(diligent, key); };
		require(token("perDays", diligent_get, month) == "33", "9 of 27 days");
		require(token("gaugeDays", diligent_get, month) == "#fUI/UIWindow.img/Quest/Gauge2/frame/25#", "a third of the gauge");
		std::string partial = "DONE=1;PT=10800;CNT=9";
		require(token("perDays", [&](const std::string& key) { return value(partial, key); }, month) == "0",
				"CNT needs the whole record");
		month.time_limit = 3600;
		require(token("perDays", diligent_get, month) == "0", "under a day, where the client divides by zero");

		// The timed titles' #Q: the time left until the end their started
		// record names.
		using ms::QuestRecordEx::expiry;
		using ms::QuestRecordEx::time_left;
		require(expiry("20090213233130") == 1234567890, "the end, in UTC");
		require(expiry("") == 3439756800, "no end is 2079");
		require(expiry("2009021323313") == 3439756800, "an end needs 14 characters");
		require(expiry("2009 2 1233x1z") == expiry("20090201230301"),
				"any 14 characters, each field read by atoi");
		require(expiry("20090229000000") == 3439756800, "no 29 February in 2009");
		require(expiry("20080229000000") == 1204243200, "but in 2008");
		require(expiry("20091301000000") == 3439756800, "no 13th month");
		require(expiry("20090101240000") == 3439756800, "no 24th hour");
		require(expiry("16000101000000") == 3439756800, "nothing before 1601");
		int64_t now = 1234567890;
		int64_t end = now + 29 * 86400 + 5 * 3600 + 7 * 60 + 9;
		require(time_left("daylimit", end, now) == "29", "the days left");
		require(time_left("hourlimit", end, now) == "5", "the hours past them");
		require(time_left("minlimit", end, now) == "7", "the minutes past those");
		require(time_left("seclimit", end, now) == "9", "the seconds past those");
		require(time_left("daylimit", now - 60, now) == "0", "time up is 0");
		require(time_left("4490", end, now) == "0", "another key is 0");

		// Diligent Explorer's #D: PT today, or dailyPlayTime once done.
		using ms::QuestRecordEx::play_time;
		std::string today = "LT=2009/02/13;DONE=0;PT=5025;CNT=9";
		auto today_get = [&](const std::string& key) { return value(today, key); };
		require(play_time("hour", today_get, 10800, 2592000) == "1", "the hours played");
		require(play_time("min", today_get, 10800, 2592000) == "23", "the minutes past them");
		require(play_time("sec", today_get, 10800, 2592000) == "45", "the seconds past those");
		require(play_time("day", today_get, 10800, 2592000) == "0", "no whole day");
		require(play_time("minhour", today_get, 10800, 2592000) == "3", "the hours to play");
		require(play_time("count", today_get, 10800, 2592000) == "9", "the days done");
		require(play_time("daylimit", today_get, 10800, 2592000) == "27", "the days needed");
		require(play_time("lose", today_get, 10800, 2592000) == "0", "another key is 0");
		std::string done = "LT=2009/02/13;DONE=1;PT=5025;CNT=9";
		auto done_get = [&](const std::string& key) { return value(done, key); };
		require(play_time("hour", done_get, 10800, 2592000) == "3", "a day done shows the whole play");
		require(play_time("min", done_get, 10800, 2592000) == "0", "and its minutes");
		std::string part = "DONE=0;PT=5025;CNT=9";
		auto part_get = [&](const std::string& key) { return value(part, key); };
		require(play_time("hour", part_get, 10800, 2592000) == "0", "PT needs the whole record");
		require(play_time("count", part_get, 10800, 2592000) == "0", "so does CNT");
		require(play_time("minhour", part_get, 10800, 2592000) == "3", "the hours to play need none");

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
