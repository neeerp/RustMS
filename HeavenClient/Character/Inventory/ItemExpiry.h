#pragma once

#include <cstdint>
#include <cstdio>
#include <string>

// The line v83 item tooltips give an item's expiry (FUN_008ebe04, report 200).
namespace ms::item_expiry
{
	// Expiry times are FILETIMEs: 100 ns ticks from 1601-01-01 UTC. Only those strictly between 1900-01-01 and
	// 2079-01-01 get a line; servers send 2079-01-01 for none.
	constexpr int64_t AFTER = 94354848000000000;
	constexpr int64_t BEFORE = 150842304000000000;
	constexpr int64_t UNIX_EPOCH = 116444736000000000;
	constexpr int64_t TICKS_PER_SECOND = 10000000;

	struct Time
	{
		int32_t year;
		int32_t month;
		int32_t day;
		int32_t hour;
		int32_t minute;
	};

	// The UTC date and time, as FileTimeToSystemTime gives it.
	inline Time utc(int64_t filetime)
	{
		int64_t seconds = (filetime - UNIX_EPOCH) / TICKS_PER_SECOND;

		if ((filetime - UNIX_EPOCH) % TICKS_PER_SECOND < 0)
			seconds--;

		int64_t days = seconds / 86400;
		int64_t rest = seconds % 86400;

		if (rest < 0)
		{
			rest += 86400;
			days--;
		}

		// Howard Hinnant's civil_from_days.
		days += 719468;
		int64_t era = (days >= 0 ? days : days - 146096) / 146097;
		int64_t doe = days - era * 146097;
		int64_t yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
		int64_t doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
		int64_t mp = (5 * doy + 2) / 153;
		int64_t day = doy - (153 * mp + 2) / 5 + 1;
		int64_t month = mp < 10 ? mp + 3 : mp - 9;
		int64_t year = yoe + era * 400 + (month <= 2 ? 1 : 0);

		return { static_cast<int32_t>(year), static_cast<int32_t>(month), static_cast<int32_t>(day),
				 static_cast<int32_t>(rest / 3600), static_cast<int32_t>(rest / 60 % 60) };
	}

	// The line for an item expiring at `filetime`, or nothing. Cash items say when they're available until,
	// sealed (locked) ones when the seal is released, the rest when they're usable until.
	inline std::string line(int64_t filetime, bool cash, bool sealed)
	{
		if (filetime <= AFTER || filetime >= BEFORE)
			return "";

		const char *format = cash ? "Available until %d/%d/%04d at %02d:%02d"
								  : sealed ? "Seal Released on %d-%d-%04d %02d:%02d"
										   : "USABLE UNTIL %d-%d-%04d AT %02d : %02d";

		Time t = utc(filetime);
		char text[64];
		std::snprintf(text, sizeof(text), format, t.month, t.day, t.year, t.hour, t.minute);

		return text;
	}

	// An item's flag for being sealed (GW_ItemSlotBase::IsProtectedItem).
	constexpr int16_t SEALED = 0x01;
}
