#pragma once

#include <chrono>
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

	// The time now as a FILETIME.
	inline int64_t now()
	{
		auto ticks = std::chrono::duration_cast<std::chrono::microseconds>(
			std::chrono::system_clock::now().time_since_epoch()).count() * 10;

		return UNIX_EPOCH + ticks;
	}

	// How far the server's clock is ahead of ours, from the time each
	// SET_FIELD ends with (v83 keeps it with the field, 0x529366).
	inline int64_t& server_offset()
	{
		static int64_t offset = 0;
		return offset;
	}

	// The server's time now as a FILETIME.
	inline int64_t server_now()
	{
		return now() + server_offset();
	}

	struct Time
	{
		int32_t year;
		int32_t month;
		int32_t day;
		int32_t hour;
		int32_t minute;
	};

	// Whether a slot open until `until` is open at `at`.
	inline bool open(int64_t until, int64_t at)
	{
		return until > at;
	}

	// The second pendant slot after `days` more, from its expiry if it is
	// still open and from `at` if not, as v83 adds them (47acdb).
	inline int64_t extended(int64_t until, int64_t at, int32_t days)
	{
		return (open(until, at) ? until : at) + static_cast<int64_t>(days) * 86400 * TICKS_PER_SECOND;
	}

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

	// The second pendant slot's name (StringPool 0x275).
	constexpr const char *PENDANT_SLOT = "PENDANT";

	// v83 passes the extra slot's dates year first, whatever the text says
	// the numbers are (0x47ad8f, 0x8f1fa2, 0x8e96c1; report b178).
	inline std::string slot_date(const char *format, const std::string &prefix, int64_t until)
	{
		Time t = utc(until);
		char text[160];
		std::snprintf(text, sizeof(text), format, t.year, t.month, t.day, t.hour, t.minute);

		return prefix + text;
	}

	// What the Cash Shop says when it opens the slot (0x145a) or puts its
	// end off (0x1459).
	inline std::string slot_notice(bool was_open, int64_t until)
	{
		if (!was_open)
			return std::string("You have added ") + PENDANT_SLOT + " slots.";

		return slot_date("slot has been extended to %d(month)-%d(date)-%d(year) at %d(hour):%d(minute).",
						 std::string("The time limit for the \\n ") + PENDANT_SLOT + " ", until);
	}

	// A slot extender's (555xxxx) line: what it would do while the slot is
	// shut (0x145b), else when the slot shuts (0x145c, 0x8f1eaa).
	inline std::string slot_extender_line(int64_t until, int64_t at)
	{
		if (!open(until, at))
			return "If you purchase this item, you will be able to equip\\nadditional equipment items.";

		return slot_date("Available until %d(month)-%d(date)-%04d(year) at %02d(hour):%02d(minute)", "", until);
	}

	// What a worn second pendant's description adds: when its effects end
	// (0x144a), or, the slot shut, what would put it on (0x144b, 0x8e9635).
	inline std::string pendant_slot_line(int64_t until, int64_t at)
	{
		if (!open(until, at))
			return std::string("The item will be equipped if you purchase a ") + PENDANT_SLOT +
				   " slot extender at the #c Cash Shop.#";

		return slot_date("#cslot expiration date of %d(month)-%d(date)-%04d(year) at %02d(hour):%02d(minute).",
						 "The item's effects will disappear after the ", until);
	}

	// An item's flag for being sealed (GW_ItemSlotBase::IsProtectedItem).
	constexpr int16_t SEALED = 0x01;

	// What a pet's Item.nx info says of its life.
	struct PetLife
	{
		bool permanent = false;
		// `limitedLife`: the seconds it lives in all. Such a pet counts down
		// the seconds it has left instead of having a date.
		int32_t limited = 0;
		// `noRevive`: the Water of Life can't bring it back once it dies.
		bool no_revive = false;
		// `life`: the days a Water of Life lasts.
		int32_t days = 0;
	};

	// Whether a pet has died (FUN_004e4003). One with limitedLife dies when its
	// seconds run out; the server marks any other dead by dating it 2079 or
	// later, so a date already past isn't enough.
	inline bool pet_dead(int64_t date_dead, int32_t remain_life, const PetLife &life)
	{
		return life.limited > 0 ? remain_life < 1 : date_dead >= BEFORE;
	}

	// The line a pet's tooltip gives its Water of Life (FUN_008ebfde), or
	// nothing for one without a date. The hour is UTC and the minutes are
	// always 00.
	inline std::string pet_line(int64_t date_dead, int32_t remain_life, const PetLife &life)
	{
		if (date_dead == 0)
			return "";

		char text[64];

		if (life.limited > 0)
		{
			std::snprintf(text, sizeof(text), "DAYS OF MAGIC : %dhrs. %dmin.", remain_life / 3600,
				remain_life / 60 % 60);

			return text;
		}

		if (pet_dead(date_dead, remain_life, life))
			return "THE WATER OF LIFE HAS DRIED UP.";

		if (life.permanent)
			return "PERMANENT";

		Time t = utc(date_dead);
		std::snprintf(text, sizeof(text), "WATER OF LIFE DRIES UP ON %d-%d-%04d AT %02d : 00", t.month, t.day,
			t.year, t.hour);

		return text;
	}

	// The line a Cash Shop commodity lasting `period` days shows (FUN_008ebe04 with the commodity's Period), or
	// nothing for one that lasts forever. Three bundles count hours instead.
	inline std::string commodity_line(int32_t item_id, int32_t period)
	{
		if (period == 0)
			return "";

		char text[64];

		if (item_id == 5211060)
			std::snprintf(text, sizeof(text), "AVAILABLE FOR %d HOURS.", 2);
		else if (item_id == 5211048 || item_id == 5360042)
			std::snprintf(text, sizeof(text), "AVAILABLE FOR %d HOURS.", 4);
		else
			std::snprintf(text, sizeof(text), "AVAILABLE FOR %d DAYS.", period);

		return text;
	}

	// The line a pet for sale shows (FUN_008ebfde with the commodity flag): its whole limitedLife, or permanent,
	// or the days its Water of Life lasts.
	inline std::string commodity_pet_line(const PetLife &life)
	{
		char text[64];

		if (life.limited > 0)
			std::snprintf(text, sizeof(text), "DAYS OF MAGIC : %dhrs. %dmin.", life.limited / 3600,
				life.limited / 60 % 60);
		else if (life.permanent)
			return "PERMANENT";
		else
			std::snprintf(text, sizeof(text), "DAYS OF MAGIC : %d", life.days);

		return text;
	}
}
