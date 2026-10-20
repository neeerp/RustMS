#pragma once

#include "MiniRoom.h"

#include "../../IO/UITypes/EnterNumber.h"

#include <cstdint>
#include <iterator>
#include <limits>
#include <optional>
#include <string>
#include <vector>

namespace ms
{
	// The parts of v83's personal shop (mini room type 4, the dialog of
	// FUN_006fa0f5) that need no UI.
	namespace personal_shop
	{
		// The owner sits in slot 0, up to three visitors after.
		constexpr uint8_t OWNER = 0;
		constexpr uint8_t SEATS = 4;

		// Level 15 and below may spend a million mesos a purchase (FUN_006fd261).
		constexpr int32_t LOW_LEVEL = 15;
		constexpr int64_t LOW_LEVEL_MESO = 1000000;

		const std::string BUNDLE_ONLY = "The number of items should be based\\non the number of items in a bundle.";

		// Shop permits, the 514 items, open a shop when double-clicked.
		inline bool is_permit(int32_t itemid)
		{
			return itemid / 10000 == 514;
		}

		// Use, setup and etc items show how many are left and sell "N for" a
		// price; equipment, throwing stars and bullets don't (FUN_006fb99d).
		inline bool shows_count(int32_t itemid)
		{
			int32_t prefix = itemid / 1000000;

			return prefix >= 2 && prefix <= 4 && !shop_list::is_rechargeable(itemid);
		}

		// A row's price: "10 for 1,500 mesos." or "1,500 mesos.".
		inline std::string price_text(const mini_room::Listing &listing)
		{
			std::string price = mini_room::meso_text(listing.price) + " mesos.";

			if (shows_count(listing.item.itemid))
				return std::to_string(listing.per_bundle) + " for " + price;

			return price;
		}

		// Prices are coloured by how many digits they have: below 100
		// thousand, a million, 10 million, 100 million, a billion, or more.
		inline size_t price_tier(int32_t price)
		{
			constexpr int32_t bounds[] = { 100000, 1000000, 10000000, 100000000, 1000000000 };
			size_t tier = 0;

			while (tier < std::size(bounds) && price >= bounds[tier])
				tier++;

			return tier;
		}

		// Why the shop closed on us (FUN_006fc731), or nothing.
		inline std::string leave_text(int8_t reason)
		{
			switch (reason)
			{
			case 1:
				return "You can't establish a miniroom right here.";
			case 3:
				return "The shop is closed.";
			case 5:
				return "The user has been banned.";
			case 6:
				return "The store cannot be opened as the time limit has passed.";
			case 0xe:
				return "The items are out of stock.";
			case 0xf:
				return "You have been banned from the room for exceeding the time limit. You are now blocked from re-entering this store.";
			default:
				return "";
			}
		}

		// Why a purchase failed (FUN_006fcd0e), or nothing.
		inline std::string buy_result_text(int8_t result)
		{
			switch (result)
			{
			case 0:
				return "";
			case 1:
				return "You don't have enough in stock.";
			case 2:
				return "You do not have enough mesos.";
			case 3:
				return "The price of the item is too high for the trade.";
			case 4:
				return "The buyer does not possess enough money to trade anymore.";
			case 5:
				return "Please check if your inventory is full or not.";
			case 6:
				return "You cannot carry more than one for this item.";
			case 7:
				return "Cannot be traded due to gender inconsistencies.";
			case 8:
				return "This item cannot be\\npurchased by anyone under 7.";
			case 9:
				return "Purchase was canceled because there was an item\\nthat disappeared because it expired.";
			case 10:
				return "A cash item purchase was canceled\\ndue to a server check.";
			case 0xb:
				return "This character is unable to do it.";
			case 0xc:
				return "The game file has been damaged\\nso you cannot participate in an item trade. Please try again\\nafter reinstalling the game.";
			case 0xe:
				return "Players that are Level 15 and below \\nmay only trade 1 million mesos per day. "
					"\\nYou have reached the limit today,\\nplease try again tomorrow.";
			default:
				return "Due to an error, the trade did not happen.";
			}
		}

		// The fee on a sale, which v83 takes off for "Total Received"
		// (FUN_006fa066): the gross times a rate, in doubles, truncated.
		inline int64_t fee(int64_t gross)
		{
			double rate = gross >= 100000000 ? 0.03
				: gross >= 25000000 ? 0.025
				: gross >= 10000000 ? 0.02
				: gross >= 5000000 ? 0.015
				: gross >= 1000000 ? 0.009
				: gross >= 100000 ? 0.004
				: 0.0;

			return static_cast<int64_t>(static_cast<double>(gross) * rate);
		}

		// Someone bought bundles of row `index` (mode 0x1a, owner only).
		struct SoldNotice
		{
			uint8_t index;
			int16_t bundles;
			std::string buyer;
		};

		inline SoldNotice read_sold(InPacket &recv)
		{
			SoldNotice sold;
			sold.index = static_cast<uint8_t>(recv.read_byte());
			sold.bundles = recv.read_short();
			sold.buyer = recv.read_string();

			return sold;
		}

		// What the owner sold since opening, for the sold-items dialog
		// (FUN_006fd066). Only the client keeps it.
		struct Sales
		{
			struct Sold
			{
				int32_t itemid;
				int32_t units;
				std::string buyer;
			};

			std::vector<Sold> sold;
			int64_t total = 0;
			int64_t received = 0;

			void add(const mini_room::Listing &listing, int16_t bundles, const std::string &buyer)
			{
				int64_t gross = static_cast<int64_t>(bundles) * listing.price;

				sold.push_back({ listing.item.itemid, bundles * listing.per_bundle, buyer });
				total += gross;
				received += gross - fee(gross);
			}
		};

		// Equipment, cash items, throwing stars and bullets, and anything with
		// a time limit sell whole, as one bundle of one (FUN_004f3144); a set
		// of stars counts as one.
		inline bool sells_whole(int32_t itemid, int64_t expiration = item_expiry::BEFORE)
		{
			int32_t prefix = itemid / 1000000;

			return prefix < 2 || prefix > 4 || shop_list::is_rechargeable(itemid) || expiration < item_expiry::BEFORE;
		}

		// What buying a cash item that sells whole asks.
		constexpr const char* CASH_BUY_QUESTION = "Cash Items become untradable\\nonce they have been traded and "
			"\\nMaple Points cannot be accumulated\\nusing Item Deletions in the Cash Shop.\\n\\nDo you really want to buy?";

		// What the owner puts up: bundles of per_bundle items, at price a bundle.
		struct Sale
		{
			int16_t bundles;
			int16_t per_bundle;
			int32_t price;
		};

		// Whether a sale's total runs past what a character can hold.
		inline bool too_pricey(int64_t bundles, int32_t price)
		{
			return bundles * price > std::numeric_limits<int32_t>::max();
		}

		// The sell dialog's boxes (FUN_006fe2c3), checked in v83's order: the
		// bundle size when selling in bundles, the count, that the count
		// fills whole bundles, then the price. A problem names the box to go
		// back to.
		struct SellForm
		{
			std::string count;
			std::string price;
			std::string per_bundle;
			bool bundled;
		};

		enum class Box
		{
			NONE,
			COUNT,
			PRICE,
			PER_BUNDLE
		};

		struct SellCheck
		{
			Box box;
			std::string problem;
			Sale sale;
		};

		inline SellCheck check_sale(const SellForm &form, int32_t itemid, int16_t quantity)
		{
			int32_t per_bundle = 1;

			if (form.bundled)
			{
				enter_number::Checked checked = enter_number::check(form.per_bundle, quantity);

				if (checked.problem != enter_number::Problem::NONE)
					return { Box::PER_BUNDLE, enter_number::problem_text(checked.problem, quantity, 1), {} };

				per_bundle = checked.number;
			}

			enter_number::Checked count = enter_number::check(form.count, quantity);

			if (count.problem != enter_number::Problem::NONE)
				return { Box::COUNT, enter_number::problem_text(count.problem, quantity, 1), {} };

			if (count.number % per_bundle != 0)
				return { Box::COUNT, BUNDLE_ONLY, {} };

			constexpr int32_t max_price = std::numeric_limits<int32_t>::max();
			enter_number::Checked price = enter_number::check(form.price, max_price);

			if (price.problem != enter_number::Problem::NONE)
				return { Box::PRICE, enter_number::problem_text(price.problem, max_price, 1), {} };

			if (sells_whole(itemid))
				return { Box::NONE, "", { 1, 1, price.number } };

			auto bundles = static_cast<int16_t>(count.number / per_bundle);

			return { Box::NONE, "", { bundles, static_cast<int16_t>(per_bundle), price.number } };
		}

		// A visitor asking for `units` of a row (FUN_006fd261): whole bundles
		// only, and no more mesos than a character can hold.
		struct Purchase
		{
			std::string problem;
			int16_t bundles;
			int64_t total;
		};

		inline Purchase purchase(const mini_room::Listing &listing, int32_t units)
		{
			if (listing.per_bundle <= 0 || units % listing.per_bundle != 0)
				return { BUNDLE_ONLY, 0, 0 };

			auto bundles = static_cast<int16_t>(units / listing.per_bundle);

			if (too_pricey(bundles, listing.price))
				return { "It has exceeded the limit that system allows.", 0, 0 };

			return { "", bundles, static_cast<int64_t>(bundles) * listing.price };
		}

		inline std::string total_question(int64_t total)
		{
			return "The total price is " + std::to_string(total) + " mesos.\\nWill you make the purchase?";
		}

		// Whether a purchase goes past the low-level limit.
		inline bool over_low_level_limit(int32_t level, int64_t total)
		{
			return level <= LOW_LEVEL && total > LOW_LEVEL_MESO;
		}
	}
}
