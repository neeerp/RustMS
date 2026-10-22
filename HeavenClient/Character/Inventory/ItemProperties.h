#pragma once

#include <cstdint>
#include <string>

// The orange lines v83 item tooltips give under an item's name, such as "One-of-a-kind Item, Untradeable" (equips
// FUN_008e8252, other items FUN_008edfc7, report 203).
namespace ms::item_properties
{
	// An equip's flags for being untradeable, and for trading once more after Scissors of Karma.
	constexpr int16_t EQUIP_UNTRADEABLE = 0x08;
	constexpr int16_t EQUIP_KARMA = 0x10;
	// An equip's flags for spikes and cold protection (GW_ItemSlotEquip vtable+8 and +0xc).
	constexpr int16_t EQUIP_SPIKES = 0x02;
	constexpr int16_t EQUIP_COLD = 0x04;
	// Any other item's flag for trading once more.
	constexpr int16_t BUNDLE_KARMA = 0x02;

	// Whether Scissors of Karma were used on the item `item_id` with `flags`, so it may trade once more.
	inline bool karma(int32_t item_id, int16_t flags)
	{
		return flags & (item_id / 1000000 == 1 ? EQUIP_KARMA : BUNDLE_KARMA);
	}

	constexpr const char *KARMA_HINT = "#cYou can enable 1 time of trading when you use the Scissors of Karma.#";

	// What an item's info node says about it.
	struct Info
	{
		bool only = false;
		bool quest = false;
		bool pquest = false;
		bool trade_block = false;
		bool equip_trade_block = false;
		bool only_equip = false;
		bool account_sharable = false;
		bool not_extend = false;
		bool trade_available = false;
		// Gone when its owner logs out.
		bool expire_on_logout = false;
	};

	// Most pieces join the first line; a few take the second when the first is in use.
	struct Lines
	{
		std::string first;
		std::string second;
	};

	inline void join(std::string &line, const std::string &text)
	{
		if (!line.empty())
			line += ", ";

		line += text;
	}

	// "Unique Equipped Item" and the like go on the first line, or else join the second.
	inline void join_free(Lines &lines, const std::string &text)
	{
		join(lines.first.empty() ? lines.first : lines.second, text);
	}

	// An untradeable item says so, or that it can trade once where Scissors of Karma were used on it, which
	// replaces the second line if the first is in use.
	inline void untradeable(Lines &lines, bool karma)
	{
		if (!karma)
			join(lines.first, "Untradeable");
		else if (lines.first.empty())
			lines.first = "Can trade 1 time (Trade disabled after transaction)";
		else
			lines.second = "Can trade 1 time (Trade disabled after transaction)";
	}

	inline void common(Lines &lines, const Info &info)
	{
		if (info.only)
			lines.first = "One-of-a-kind Item";

		if (info.quest)
			join(lines.first, "Quest Item");

		if (info.pquest)
			join(lines.first, "Party Quest Item");
	}

	// The lines of an equip with `flags`, which `expires` where it has an expiry line.
	inline Lines equip(const Info &info, int16_t flags, bool expires)
	{
		Lines lines;
		bool flagged = flags & EQUIP_UNTRADEABLE;

		common(lines, info);

		if (info.trade_block || flagged)
			untradeable(lines, flags & EQUIP_KARMA);

		if (info.equip_trade_block && !flagged)
			join(lines.first, "Trade disabled when equipped");

		if (info.only_equip)
			join_free(lines, "Unique Equipped Item");

		if (info.account_sharable)
			join(lines.first, "Movement is possible only within the account");

		if (info.not_extend && expires)
			join_free(lines, "Time limit extension is not possible.");

		return lines;
	}

	// The lines of any other item, with `flags` 0 where the player doesn't have it.
	inline Lines bundle(const Info &info, int16_t flags)
	{
		Lines lines;

		common(lines, info);

		if (info.trade_block)
			untradeable(lines, flags & BUNDLE_KARMA);

		return lines;
	}

	// A description with the hint that Scissors of Karma would let the item trade, on a line of its own.
	inline std::string with_karma_hint(std::string desc)
	{
		if (!desc.empty())
			desc += "\\n";

		return desc + KARMA_HINT;
	}
}
