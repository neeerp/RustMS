#pragma once

#include <cstdint>

namespace ms
{
	// The Scissors of Karma (5520000, v83 cash type 0x3f, report c010). Double-clicked in the Cash tab they open a
	// window an item is dropped on from the Item window; OK sends them with the item's tab and slot, and the item may
	// then change hands once.
	namespace karma_scissors
	{
		constexpr int32_t KARMA_SCISSORS = 5520000;

		// An item dropped on the window.
		struct Target
		{
			int32_t item_id = 0;
			// Item flag 0x01.
			bool sealed = false;
			// Item.nx `info/tradeAvailable`.
			bool trade_available = false;
			// Equipment flag 0x10, or 0x02 for others.
			bool karma = false;
			// Equipment flag 0x08: an equip whose Karma a trade used up, or bound by wearing it.
			bool bound = false;
		};

		enum class Drop
		{
			ACCEPTED,
			// StringPool 0xd6d.
			SEALED,
			// StringPool 0x15c4.
			NOT_USABLE,
			// StringPool 0x1237.
			UNLOCKED
		};

		// v83 0x830ef7, in its order. v83 takes a bound equip, which the server refuses; HeavenClient refuses it
		// too rather than say the scissors were used.
		inline Drop check(const Target &target)
		{
			if (target.sealed)
				return Drop::SEALED;

			if (!target.trade_available)
				return Drop::NOT_USABLE;

			if (target.karma)
				return Drop::UNLOCKED;

			if (target.bound)
				return Drop::NOT_USABLE;

			return Drop::ACCEPTED;
		}

		inline const char *refusal(Drop drop)
		{
			switch (drop)
			{
				case Drop::SEALED:
					return "You must break the seal first.";
				case Drop::NOT_USABLE:
					return "This item cannot be used.";
				case Drop::UNLOCKED:
					return "This item has already been unlocked.";
				default:
					return "";
			}
		}

		// StringPool 0x1238, in the chat once the scissors are sent.
		constexpr const char *USED = "You have used the Scissors of Karma, so 1 time of trading has been enabled.";
	}
}
