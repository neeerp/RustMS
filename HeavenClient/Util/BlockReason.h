#pragma once

#include "../Character/Inventory/ItemExpiry.h"

#include <cstdint>
#include <cstdio>
#include <string>

// v83's notice for a blocked account (LOGIN_STATUS result 2, 0x5f83ee).
namespace ms::block_reason
{
	// The BlockReason.img node for a reason byte: 21 to 40 share 12's text.
	inline int32_t node(uint8_t reason)
	{
		return reason > 20 && reason < 41 ? 12 : reason;
	}

	// When the block ends, as v83 writes it, or that it never does when that is three or more years
	// away. The time is the FILETIME as sent, without a time zone.
	inline std::string until(int64_t filetime, int32_t this_year)
	{
		if (item_expiry::utc(filetime).year - this_year >= 3)
			return "The ID has been permanently blocked.\\nSo you won't be able to use this account.";

		static const char *months[] = { "Jan", "Feb", "Mar", "Apr", "May", "Jun",
										"Jul", "Aug", "Sep", "Oct", "Nov", "Dec" };

		item_expiry::Time time = item_expiry::utc(filetime);
		int32_t hour = time.hour < 13 ? time.hour : time.hour % 12;

		char text[64];
		std::snprintf(text, sizeof(text), "You can login after\\n%s/%2d/%4d %2d:%02d %s", months[(time.month - 1) % 12],
					  time.day, time.year, hour, time.minute, time.hour > 12 ? "pm" : "am");

		return text;
	}

	// The whole text, given the node's msg ("" when BlockReason.img has none). Nothing means the plain
	// "blocked" notice.
	inline std::string text(uint8_t reason, int64_t filetime, int32_t this_year, const std::string &msg)
	{
		if (reason == 99)
			return "You have been blocked for typing in an invalid password or pincode 5 times.\\n"
				+ until(filetime, this_year);

		if (reason == 0 || msg.empty())
			return "";

		return msg + "\\n" + until(filetime, this_year);
	}
}
