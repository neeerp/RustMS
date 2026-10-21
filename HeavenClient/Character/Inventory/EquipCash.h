#pragma once

#include "ItemExpiry.h"

#include <cstdint>
#include <cstdio>
#include <string>

namespace ms
{
	// Cash items dropped from the Cash tab on a worn equip (FUN_004f4bb7, report c009): the Item Tag (5060000)
	// prints its user's name on the equip, a Magical Sandglass (550xxxx) adds to the time a time-limited one lasts.
	namespace equip_cash
	{
		// v83 0x4f4f0a: 5060xxx ending in 0. Only 5060000 exists.
		inline bool is_tag(int32_t item_id)
		{
			return item_id / 1000 == 5060 && item_id % 10 == 0;
		}

		inline bool is_sandglass(int32_t item_id)
		{
			return item_id / 10000 == 550;
		}

		// StringPool 0xcd6 and 0xcd7 (sic).
		constexpr const char *TAGGED = "The item already has a printed name on it..";

		inline std::string tag_question(const std::string &equip_name)
		{
			return "Woulid like to print your name on " + equip_name + "?";
		}

		// A sandglass's Item.nx info.
		struct Sandglass
		{
			// `addTime`: the seconds it adds.
			int32_t add_seconds = 0;
			// `maxDays`: how far from now the date may go, 0 for any.
			int32_t max_days = 0;
		};

		// A worn equip's state the sandglass looks at.
		struct Target
		{
			int64_t expiration = 0;
			bool cash = false;
			bool sealed = false;
			// Character.nx `info/notExtend`.
			bool not_extend = false;
		};

		enum class Extend
		{
			// Nothing happens: a permanent, cash or sealed equip.
			NONE,
			// StringPool 0x147a.
			NOT_EXTENDABLE,
			// StringPool 0x11d4.
			TOO_FAR,
			// The question StringPool 0x11d3, then the request.
			ASK
		};

		// v83 0x4f5080-0x4f51c6, in its order.
		inline Extend extend(const Target &target, const Sandglass &sandglass, int64_t now)
		{
			if (target.expiration >= item_expiry::BEFORE || target.cash || target.sealed)
				return Extend::NONE;

			if (target.not_extend)
				return Extend::NOT_EXTENDABLE;

			int64_t extended = target.expiration + sandglass.add_seconds * item_expiry::TICKS_PER_SECOND;
			int64_t latest = now + static_cast<int64_t>(sandglass.max_days) * 86400 * item_expiry::TICKS_PER_SECOND;

			if (sandglass.max_days > 0 && extended > latest)
				return Extend::TOO_FAR;

			return Extend::ASK;
		}

		constexpr const char *NOT_EXTENDABLE = "Time limit extension is not possible on this item.";

		inline std::string too_far(int32_t max_days)
		{
			return "You cannot extend the effective date beyond " + std::to_string(max_days) + " days.";
		}

		// The time a sandglass adds, as StringPool 0x11d5-0x11d7 put it. Which one v83 picks is unproven: every
		// sandglass adds whole days.
		inline std::string added(int32_t seconds)
		{
			char text[32];

			if (seconds % 86400 == 0)
				std::snprintf(text, sizeof(text), "%d days", seconds / 86400);
			else if (seconds % 3600 == 0)
				std::snprintf(text, sizeof(text), "%d hrs.", seconds / 3600);
			else
				std::snprintf(text, sizeof(text), "%d min.", seconds / 60);

			return text;
		}

		// StringPool 0x11d3, with its two spaces.
		inline std::string extend_question(const std::string &equip_name, int32_t seconds)
		{
			return "When used on " + equip_name + ",  the effective date will be extended by " + added(seconds) +
				   ". Would you like to use it?";
		}
	}
}
