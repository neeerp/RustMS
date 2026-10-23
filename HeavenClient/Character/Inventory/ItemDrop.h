#pragma once

#include "ItemExpiry.h"
#include "ItemProperties.h"

#include <cstdint>
#include <vector>

namespace ms
{
	// What the v83 client checks before an item leaves the inventory for the
	// ground (FUN_004f31a5), a trade (FUN_004f3ef4) or a personal shop
	// (FUN_004f41b9).
	namespace item_drop
	{
		constexpr const char *UNAVAILABLE = "This item is unavailable to trade with others.";
		constexpr const char *SEALED = "Sealed items cannot be\\nsold, traded, or dropped.";

		// What those checks weigh.
		struct Traits
		{
			// A cash item or one with a Cash Shop serial.
			bool cash = false;
			bool quest = false;
			bool pquest = false;
			// tradeBlock, or equipment made untradeable (FUN_005d28ae).
			bool untradeable = false;
			// Scissors of Karma let it trade once.
			bool karma = false;
			// accountSharable: it moves only through storage.
			bool account = false;
			bool sealed = false;
			// It has an expiry, which makes a stack go whole (FUN_004f3144).
			bool expires = false;
		};

		inline Traits traits(const item_properties::Info &info, bool equip, bool cash, int16_t flags,
			int64_t expiration)
		{
			Traits traits;
			traits.cash = cash;
			traits.quest = info.quest;
			traits.pquest = info.pquest;
			traits.untradeable = info.trade_block || (equip && (flags & item_properties::EQUIP_UNTRADEABLE));
			traits.karma = flags & (equip ? item_properties::EQUIP_KARMA : item_properties::BUNDLE_KARMA);
			traits.account = info.account_sharable;
			traits.sealed = flags & item_expiry::SEALED;
			traits.expires = expiration < item_expiry::BEFORE;

			return traits;
		}

		// Wedding rings and a few event items, which v83 keeps without a word.
		inline bool kept(int32_t item_id)
		{
			switch (item_id)
			{
				case 1112803:
				case 1112806:
				case 1112807:
				case 1112809:
				case 3994085:
				case 4031373:
				case 4031374:
				case 4031375:
				case 4031376:
				case 4031480:
				case 4031481:
					return true;
				default:
					return item_id / 10000 == 224;
			}
		}

		// Cash items, which include anything with a Cash Shop serial, and
		// Flowerpots (4220xxx) can't be dropped.
		inline bool refused(int32_t item_id, bool cashitem)
		{
			return cashitem || item_id / 1000 == 4220;
		}

		// A Yes/No question, or a notice that ends the drop.
		struct Step
		{
			bool question;
			const char *text;
		};

		// What v83 puts up in turn before dropping an item it doesn't refuse.
		// Yes goes on to the next.
		inline std::vector<Step> steps(int32_t item_id, const Traits &item)
		{
			constexpr const char *lost = "This item can't be taken back once\\nthrown away. Will you still drop it?";

			std::vector<Step> steps;
			bool card = item_id / 10000 == 430;

			if (item.quest && !item.karma)
				steps.push_back({ true, lost });

			// So a quest item that is untradeable too asks twice.
			if (!card && item.untradeable)
				steps.push_back({ true, item.karma
					? "Once dropped, the item cannot be traded.\\nWould you still like to drop it?" : lost });

			if (item.account)
				steps.push_back({ true, "This item cannot be recovered once dropped. However, It is possible to "
					"transfer it to a different character on the same account by using the storage system. "
					"\\n\\nDo you really want to drop this item?" });

			if (item.sealed)
				steps.push_back({ false, SEALED });
			else if (card)
				steps.push_back({ true, "Would you like to throw\\naway this card?" });

			return steps;
		}

		// Whether to ask how many to drop. Equipment, cash items, throwing
		// stars and bullets go as one, as does anything with an expiry; the
		// server throws a whole stack of stars or bullets.
		inline bool asks_count(int32_t item_id, int16_t count, bool expires = false)
		{
			int32_t prefix = item_id / 1000000;
			int32_t kind = item_id / 10000;

			return count >= 2 && !expires && prefix >= 2 && prefix <= 4 && kind != 207 && kind != 233;
		}

		// Whether an item may be put up in a trade (FUN_007c359f).
		inline bool tradeable(const Traits &item)
		{
			return !item.sealed && !item.cash && (!item.quest || item.karma) && (!item.untradeable || item.karma)
				&& !item.account;
		}

		// Why an item can't be put up in a trade (FUN_004f3ef4), or nothing: a
		// sealed item says so before anything else.
		inline const char *trade_refusal(const Traits &item)
		{
			if (item.sealed)
				return SEALED;

			return tradeable(item) ? nullptr : UNAVAILABLE;
		}

		// Why an item can't be put up in a personal shop (FUN_006fd96c), or nothing.
		inline const char *unsellable(const Traits &item)
		{
			if (item.sealed)
				return SEALED;

			if (item.pquest || item.account || (!item.karma && (item.cash || item.quest || item.untradeable)))
				return UNAVAILABLE;

			return nullptr;
		}

		// Whether Duey takes an item to send (FUN_004f483e), once it isn't
		// sealed: Karma lets a quest or untradeable item go, but nothing with
		// an expiry.
		inline bool sendable(const Traits &item)
		{
			return !item.cash && !item.expires && !item.pquest && !item.account && (!item.quest || item.karma)
				&& (!item.untradeable || item.karma);
		}

		// Whether the storage keeper lists an item to store (FUN_007c6e3a).
		// Account items are what storage is for, and Karma lets the rest in.
		inline bool storable(const Traits &item)
		{
			return !item.cash && (!item.quest || item.karma) && (!item.untradeable || item.karma);
		}

		// Whether the Wedding Registry lists an item to give (0x9d877b):
		// what storage takes, less account items.
		inline bool giftable(const Traits &item)
		{
			return storable(item) && !item.account;
		}
	}
}
