#pragma once

#include <algorithm>
#include <cstdint>
#include <cstdlib>
#include <climits>
#include <functional>
#include <string>

namespace ms
{
	// The records the client keeps beside the quest log ("quest record ex"),
	// each "k=v;k=v", and the Quest Log's #j tokens that read them.
	namespace QuestRecordEx
	{
		// The value under `key` in `raw`, or "": split on ';', then each pair
		// on '=' (0x4e7856).
		inline std::string value(const std::string& raw, const std::string& key)
		{
			size_t start = 0;

			while (start <= raw.size())
			{
				size_t end = raw.find(';', start);

				if (end == std::string::npos)
					end = raw.size();

				std::string pair = raw.substr(start, end - start);
				size_t eq = pair.find('=');

				if (eq != std::string::npos && pair.substr(0, eq) == key)
					return pair.substr(eq + 1);

				start = end + 1;
			}

			return "";
		}

		// A count against a quest's need for it, which the titles' tokens show
		// as the count, a percentage and a gauge: `share` is the count's part
		// of `whole`, clamped to 0..whole, and 0 with no need.
		struct Tally
		{
			int32_t count = 0;
			int32_t need = 0;

			int32_t share(int32_t whole) const
			{
				return need <= 0 ? 0 : std::clamp(count * whole / need, 0, whole);
			}
		};

		// What the titles' tokens read beside the record: the party quests at
		// rank S against partyQuest_S (PQ Mania), the quests completed as the
		// client counts them against questComplete (Quest Specialist), the
		// character's level and fame, and the quest's QuestInfo timeLimit2 in
		// seconds (Diligent Explorer's days).
		struct Counts
		{
			Tally rank_s;
			Tally completed;
			int32_t level = 0;
			int32_t fame = 0;
			int32_t time_limit = 0;
		};

		// Diligent Explorer's day in its record: whether the day's play is
		// done, the seconds played (`PT`) and the days done (`CNT`), read
		// only when `LT`, `DONE`, `PT` and `CNT` are all there, as a byte and
		// two words (0x4e6b2f).
		struct Daily
		{
			uint8_t done = 0;
			uint16_t played = 0;
			uint16_t count = 0;
		};

		inline Daily daily(const std::function<std::string(const std::string&)>& get)
		{
			if (get("LT").empty() || get("DONE").empty() || get("PT").empty() || get("CNT").empty())
				return {};

			return {
				static_cast<uint8_t>(std::atoi(get("DONE").c_str())),
				static_cast<uint16_t>(std::atoi(get("PT").c_str())),
				static_cast<uint16_t>(std::atoi(get("CNT").c_str()))
			};
		}

		// The days a title's record counts toward its need: its `CNT`, and
		// the need, nine tenths of timeLimit2's whole days.
		inline Tally days(const std::function<std::string(const std::string&)>& get, int32_t time_limit)
		{
			return { daily(get).count, static_cast<int32_t>((time_limit / 86400) * 0.9) };
		}

		// The Unix time a timed title's started record names as its end, the
		// 14 digits "yyyyMMddHHmmss" in UTC (0x4e6a03), or 2079-01-01 for any
		// other record.
		inline int64_t expiry(const std::string& progress)
		{
			const int64_t NEVER = 3439756800;

			if (progress.size() != 14 || !std::all_of(progress.begin(), progress.end(), [](char c) { return c >= '0' && c <= '9'; }))
				return NEVER;

			auto part = [&](size_t at, size_t length) { return std::atoi(progress.substr(at, length).c_str()); };
			int64_t year = part(0, 4) - (part(4, 2) <= 2 ? 1 : 0);
			int64_t month = part(4, 2);
			// Howard Hinnant's days_from_civil.
			int64_t era = (year >= 0 ? year : year - 399) / 400;
			int64_t year_of_era = year - era * 400;
			int64_t day_of_year = (153 * (month > 2 ? month - 3 : month + 9) + 2) / 5 + part(6, 2) - 1;
			int64_t day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
			int64_t days = era * 146097 + day_of_era - 719468;

			return days * 86400 + part(8, 2) * 3600 + part(10, 2) * 60 + part(12, 2);
		}

		// A #Q token: the time left until `expiry`, never below 0, as whole
		// days (`daylimit`) or the hours, minutes or seconds past them, "%d";
		// any other key prints 0 (0x9a1189).
		inline std::string time_left(const std::string& key, int64_t expiry, int64_t now)
		{
			auto left = static_cast<int32_t>(std::clamp<int64_t>(expiry - now, 0, INT32_MAX));

			if (key == "daylimit")
				return std::to_string(left / 86400);

			if (key == "hourlimit")
				return std::to_string(left / 3600 % 24);

			if (key == "minlimit")
				return std::to_string(left / 60 % 60);

			if (key == "seclimit")
				return std::to_string(left % 60);

			return "0";
		}

		// A #D token (0x9a13cc): the day's play, `PT`, as `day`, `hour`,
		// `min` or `sec`, or all of QuestInfo dailyPlayTime once the day is
		// done or when the key starts with "min" (`minhour`, the hours to
		// play); `count` the days done and `daylimit` their need. Any other
		// key prints 0.
		inline std::string play_time(std::string key, const std::function<std::string(const std::string&)>& get,
			int32_t daily_play_time, int32_t time_limit)
		{
			Daily day = daily(get);
			int32_t seconds = day.played;

			if (key.size() > 3 && key.compare(0, 3, "min") == 0)
			{
				key = key.substr(3);
				seconds = daily_play_time;
			}
			else if (day.done != 0)
			{
				seconds = daily_play_time;
			}

			if (key == "day")
				return std::to_string(seconds / 86400);

			if (key == "hour")
				return std::to_string(seconds / 3600 % 24);

			if (key == "min")
				return std::to_string(seconds / 60 % 60);

			if (key == "sec")
				return std::to_string(seconds % 60);

			if (key == "daylimit")
				return std::to_string(days(get, time_limit).need);

			if (key == "count")
				return std::to_string(day.count);

			return "0";
		}

		// The gauge #jgaugePqS# draws: the 79x8 frame and up to 75 one-pixel
		// slices from (2, 3), passed as a path ending in the slices.
		const std::string GAUGE = "UI/UIWindow.img/Quest/Gauge2/frame/";
		const std::string GAUGE_SLICE = "UI/UIWindow.img/Quest/Gauge2/gauge";
		constexpr int32_t GAUGE_SLICES = 75;

		// The slices a gauge's path asks for, or -1 for another path.
		inline int32_t gauge_slices(const std::string& path)
		{
			if (path.rfind(GAUGE, 0) != 0 || path.size() == GAUGE.size())
				return -1;

			std::string digits = path.substr(GAUGE.size());

			if (!std::all_of(digits.begin(), digits.end(), [](char c) { return c >= '0' && c <= '9'; }) || digits.size() > 2)
				return -1;

			return std::min(std::atoi(digits.c_str()), GAUGE_SLICES);
		}

		// What the #j token for `key` shows, `get` reading the quest's record,
		// as the client's quest text formatter (0x99dd54) writes it: counts
		// "%4d" (the seconds "%2d"), the give-ups the tries less the rest, a
		// reward "Acquire" or "Fail", the rank as its image (F if none), PQ
		// Mania's S ranks and Quest Specialist's quests as a count, a
		// percentage and a gauge of the need, the level and fame "%d", and
		// anything else as it stands. The other titles read their own
		// record: Outstanding Citizen the fame gained since `popS` against
		// `popG`, Veteran Hunter `mon` against `mg`, and Diligent Explorer
		// its days. A missing or zero need counts as 1, but where the client
		// would divide by zero (perPop's popG "0", a timeLimit2 under a
		// day) this shows 0.
		inline std::string token(const std::string& key, const std::function<std::string(const std::string&)>& get,
			Counts counts = {})
		{
			auto gauge = [](Tally tally) { return "#f" + GAUGE + std::to_string(tally.share(GAUGE_SLICES)) + "#"; };
			auto number = [&](const std::string& k) { return std::atoi(get(k).c_str()); };
			auto at_least_one = [](int32_t n) { return n == 0 ? 1 : n; };
			int32_t gained = counts.fame - number("popS");

			if (key == "popgap")
				return std::to_string(gained);

			if (key == "perPop")
				return std::to_string(Tally { gained, get("popG").empty() ? 1 : number("popG") }.share(100));

			if (key == "gaugePop")
				return gauge({ gained, at_least_one(number("popG")) });

			if (key == "perHunt")
				return std::to_string(Tally { number("mon"), at_least_one(number("mg")) }.share(100));

			if (key == "gaugeHunt")
				return gauge({ number("mon"), at_least_one(number("mg")) });

			if (key == "perDays")
				return std::to_string(days(get, counts.time_limit).share(100));

			if (key == "gaugeDays")
				return gauge(days(get, counts.time_limit));

			if (key == "scnt")
				return std::to_string(counts.rank_s.count);

			if (key == "perPqS")
				return std::to_string(counts.rank_s.share(100));

			if (key == "gaugePqS")
				return gauge(counts.rank_s);

			if (key == "cmpcnt")
				return std::to_string(counts.completed.count);

			if (key == "perQCnt")
				return std::to_string(counts.completed.share(100));

			if (key == "gaugeQCnt")
				return gauge(counts.completed);

			if (key == "level")
				return std::to_string(counts.level);

			if (key == "pop")
				return std::to_string(counts.fame);

			auto padded = [](int32_t n, size_t width)
			{
				std::string digits = std::to_string(n);
				return std::string(digits.size() < width ? width - digits.size() : 0, ' ') + digits;
			};

			if (key == "rank")
			{
				std::string rank = get(key);
				return "#fUI/Basic.img/PQuestRank/" + (rank.empty() ? std::string("F") : rank) + "#";
			}

			if (key == "try" || key == "vic" || key == "cmp" || key == "lose" || key == "draw" || key == "min")
				return padded(number(key), 4);

			if (key == "sec")
				return padded(number(key), 2);

			if (key == "gvup")
				return std::to_string(number("try") - number("vic") - number("lose") - number("draw"));

			if (key.rfind("have", 0) == 0)
				return number(key) > 0 ? "Acquire" : "Fail";

			return get(key);
		}
	}
}
