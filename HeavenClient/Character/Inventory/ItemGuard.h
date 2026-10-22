#pragma once

#include "ItemExpiry.h"

#include <algorithm>
#include <cstdint>
#include <cstdio>
#include <string>

namespace ms
{
	// The Item Guards (v83 CUIItemProtector, report c009): 5060001 seals an item for good, 5061xxx for their
	// `protectTime` days. Double-clicked in the Cash tab, one opens a window an item is dropped on; OK asks, and Yes
	// sends the guard with the item's tab and slot. A timed seal's end is the item's date.
	namespace item_guard
	{
		constexpr int32_t ITEM_GUARD = 5060001;

		inline bool is_guard(int32_t item_id)
		{
			return item_id == ITEM_GUARD || item_id / 1000 == 5061;
		}

		// An item dropped on the window.
		struct Target
		{
			int32_t item_id = 0;
			// From the Equip tab.
			bool equip_tab = false;
			bool sealed = false;
			int64_t expiration = 0;
			// Item.nx `info/expireOnLogout`.
			bool expire_on_logout = false;
			// Bought in the Cash Shop: it has a serial.
			bool cash = false;
		};

		enum class Drop
		{
			// StringPool 0xd59.
			REFUSED,
			ACCEPTED,
			// Taken, with StringPool 0xd5b.
			WARNED
		};

		// Whether the item has a date short of 2079.
		inline bool timed(int64_t expiration)
		{
			return expiration > item_expiry::AFTER && expiration < item_expiry::BEFORE;
		}

		// v83 0x4f434f-0x4f449e and 0x82a5e9: a timed seal may be extended, but a time-limited item, a cash one or
		// one gone at logout can't be sealed. Of the others only equipment, stars, bullets and 22500xx can. The
		// potions a pet uses are taken with a warning.
		inline Drop check(const Target &target)
		{
			if (target.sealed != timed(target.expiration))
				return Drop::REFUSED;

			if (target.expire_on_logout || target.cash || target.item_id / 100 == 11123)
				return Drop::REFUSED;

			int32_t kind = target.item_id / 10000;

			if (!target.equip_tab && kind != 207 && kind != 233 && target.item_id / 100 != 22500)
				return Drop::REFUSED;

			switch (kind)
			{
				case 200:
				case 201:
				case 202:
				case 205:
				case 221:
				case 236:
				case 238:
				case 245:
					return Drop::WARNED;
				default:
					return Drop::ACCEPTED;
			}
		}

		// When a timed guard of `days` would end the target's seal: that many days from its seal's end, or from
		// `now`.
		inline int64_t sealed_until(const Target &target, int32_t days, int64_t now)
		{
			int64_t from = target.sealed && timed(target.expiration) ? std::max(target.expiration, now) : now;

			return from + static_cast<int64_t>(days) * 86400 * item_expiry::TICKS_PER_SECOND;
		}

		// StringPool 0xd59, 0xd5a and 0xd5b.
		constexpr const char *REFUSED = "This item cannot be sealed.";
		constexpr const char *NO_ITEM = "Please select the item you'd like to seal.";
		constexpr const char *WARNING = "This item cannot be used\\nonce it is sealed.";

		// StringPool 0xd5c.
		inline std::string question(const std::string &name)
		{
			return name + "\\nWould you like to seal this item?";
		}

		// StringPool 0x1395, or 0x1396 for a seal already timed, with the date in UTC as the tooltip's.
		inline std::string timed_question(const std::string &name, int64_t until, bool extend)
		{
			const char *format = extend
				? "%s\\n will extend the item seal until %d-%d-%04d %02d:%02d.\\n Do you want to proceed?"
				: "%s\\n will be sealed until %d-%d-%04d %02d:%02d.\\nDo you want to proceed?";

			item_expiry::Time t = item_expiry::utc(until);
			char text[256];
			std::snprintf(text, sizeof(text), format, name.c_str(), t.month, t.day, t.year, t.hour, t.minute);

			return text;
		}
	}
}
