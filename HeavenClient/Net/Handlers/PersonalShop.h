#pragma once

#include "MiniRoom.h"

#include "../../IO/UITypes/EnterNumber.h"
#include "../../Util/CharacterName.h"

#include <algorithm>
#include <cctype>
#include <cstdint>
#include <cstdio>
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

	// The hired merchant's differences (mini room type 5, the dialog of
	// 0x517eca, which builds on the personal shop's).
	namespace hired_merchant
	{
		// Hired merchant permits, the 503 items, ask the server first
		// (0xa27b7d), and only for characters over level 15.
		inline bool is_permit(int32_t itemid)
		{
			return itemid / 10000 == 503;
		}

		constexpr uint16_t MIN_LEVEL = 15;

		// The answers of ENTRUSTED_SHOP_CHECK_RESULT (0xa27d75) with more
		// to them than a notice.
		enum Check : int8_t
		{
			CHECK_OK = 7,
			OPEN_ELSEWHERE = 8,
			UNKNOWN_INT = 0xd,
			RENAMED = 0xe,
			MESSAGE = 0x12
		};

		// The notice of an answer, or nothing.
		inline std::string check_text(int8_t code)
		{
			switch (code)
			{
			case 9:
				return "Please use this after retrieving items\\n from Fredrick of Free Market.";
			case 0xa:
				return "Another character is currently using the item.\\nPlease log on as a different character and close\\n"
					"the store, or empty out the Store Bank.";
			case 0xb:
				return "You are currently unable to open the store.";
			case 0xf:
				return "Please retrieve your items from Fredrick.";
			default:
				return "";
			}
		}

		// A store already open: the channel's name, and the Free Market
		// room that the map id ends in.
		inline std::string open_elsewhere_text(const std::string &channel, int32_t mapid)
		{
			return "Your store is currently open\\nin Channel " + channel + ", Free Market " + std::to_string(mapid % 100) +
				".\\nPlease use this after closing the store.";
		}

		// A GM renaming a store.
		inline std::string rename_text(bool renamed)
		{
			return renamed ? "Renaming Successful." : "Renaming Failed - Can't find the Hired Merchant.";
		}

		// Fredrick's contract (0xa1dff8), shown in an NPC dialog once the
		// store has a name.
		constexpr int32_t FREDRICK = 9030000;

		const std::string CONTRACT = "Hello. I'm Fredrick, the Union Chief of the Hired Merchant Union. The way to use Hired Merchants "
			"is really not that different from using a regular store, so you need not worry. There are a few things you'll need "
			"to be aware of, however, so please read this contract of agreement VERY carefully before hiring the Merchants.\r\n\r\n"
			"Hired Merchant Contract\r\n"
			"1) According to the points stated below, Hired Merchants disappear after storing your items and mesos at the Store Bank.\r\n"
			"  - If the Hired Merchant works at the same spot for more than #r24 Hours#k straight, #rthe Hired Merchant's contract "
			"will expire, and the Merchant will disappear.#k\r\n"
			"  - #rIf the inventory is full#k, the Hired Merchant will be removed from its post (executes after 1st warning)\r\n"
			"2) After Hired Merchant disappears, you'll need to see #rFredrick#k, who's at the Free Market Entrance, and retrieve "
			"your items and mesos.\r\n"
			"3) You will not be able to hire Merchants until you have retrieved all of your mesos and items.\r\n"
			"4) A 1% storing fee of the total sales and original price of the item will be charged daily to the user storing "
			"the mesos and items at the Store Bank past #r24 hours#k.\r\n"
			"5) Once the storing fee exceeds 100%, the mesos and items stored will be donated to the Free Market Fund.\r\n"
			"6) GM's hold the right to change the name of the store without notice if it contains vulgar, inappropriate language.\r\n"
			"7) Players can have only one Hired Merchant set up at a time.";

		// The owner's list of visitors (mode 0x2e): who came in, and how
		// many milliseconds ago.
		struct Visitor
		{
			std::string name;
			uint32_t ms;
		};

		inline std::vector<Visitor> read_visitors(InPacket &recv)
		{
			std::vector<Visitor> visitors(static_cast<uint16_t>(recv.read_short()));

			for (Visitor &visitor : visitors)
			{
				visitor.name = recv.read_string();
				visitor.ms = static_cast<uint32_t>(recv.read_int());
			}

			return visitors;
		}

		// The list shows ten (0x51a8b3).
		constexpr size_t VISITOR_ROWS = 10;

		// " 1hrs 5min" (0x51a565).
		inline std::string visit_time_text(uint32_t ms)
		{
			return " " + std::to_string(ms / 3600000) + "hrs " + std::to_string(ms % 3600000 / 60000) + "min";
		}

		// The owner's blacklist (mode 0x2f).
		inline std::vector<std::string> read_blacklist(InPacket &recv)
		{
			std::vector<std::string> names(static_cast<uint16_t>(recv.read_short()));

			for (std::string &name : names)
				name = recv.read_string();

			return names;
		}

		constexpr size_t BLACKLIST_MAX = 20;
		// The longest name the add prompt takes.
		constexpr size_t BLACKLIST_NAME_MAX = 12;

		inline std::string trimmed(const std::string &text)
		{
			size_t first = text.find_first_not_of(' ');

			if (first == std::string::npos)
				return "";

			return text.substr(first, text.find_last_not_of(' ') - first + 1);
		}

		// Why a typed name can't be added (0x519bac), in v83's order, or
		// nothing. The client only checks how the name is spelled; myself
		// matches exactly, a listed name in any case.
		inline std::string blacklist_problem(const std::string &name, const std::string &own,
			const std::vector<std::string> &listed)
		{
			if (!character_name_valid(name))
				return "This character name does not exist\\nPlease check again";

			if (name == own)
				return "You may not enter yourself in the Blacklist";

			auto lower = [](std::string text)
			{
				std::transform(text.begin(), text.end(), text.begin(), [](unsigned char c) { return std::tolower(c); });

				return text;
			};

			for (const std::string &other : listed)
				if (lower(other) == lower(name))
					return "This name has already been registered\\nPlease check again";

			return "";
		}

		// Why the merchant closed on us (0x518d5f), and whether the window
		// goes; v83 keeps it open, with an empty notice, for anything else.
		struct Leave
		{
			std::string text;
			bool closes;
		};

		inline Leave leave(int8_t reason)
		{
			switch (reason)
			{
			case 6:
				return { "The store cannot be opened as the time limit has passed.", true };
			case 0x11:
				return { "The owner of the store is currently\\nundergoing store maintenance.\\nPlease try again in a bit.", true };
			case 0x12:
				return { "It is past the working hours,\\nso the store will close.", true };
			case 0x13:
				return { "The use of the Remote Control stopped\\ndue to moving to another map. Please use it later.", true };
			case 0x14:
				return { "Your shop was closed at the discretion of a GM. Please see Fred to recover your items.", true };
			default:
				return { "", false };
			}
		}

		// How closing the store went (0x5190e1).
		inline std::string close_text(int8_t code)
		{
			switch (code)
			{
			case 0:
				return "Both the items and mesos\\nare successfully retrieved.";
			case 1:
				return "Unable to retrieve the items and mesos due to the store having too much money.\\n"
					"Please go see Fredrick at the Free Market entrance.";
			case 2:
				return "Unable to retrieve the items because one of the items ___________, but the Mesos were retrieved.\\n"
					"Please go see Fredrick at the Free Market entrance.";
			case 3:
				return "Unable to retrieve the items due to full inventory, but the Mesos were retrieved.\\n"
					"the item could not be retrieved.\\nPlease go see Fredrick at the Free Market entrance.";
			case 4:
				return "Unable to retrieve the items for unknown reasons, but the Mesos were retrieved.\\n"
					"Please go see Fredrick at the Free Market entrance.";
			default:
				return "";
			}
		}

		// The owner's countdown (0x518314): a store lives 1440 minutes from
		// opening. Nothing outside its last day.
		inline std::string countdown_text(int64_t elapsed_ms)
		{
			auto left = static_cast<int32_t>(1439 - elapsed_ms / 60000);
			int32_t hours = left / 60;

			if (left < 0 || hours > 23)
				return "";

			char text[32];
			std::snprintf(text, sizeof(text), "Store Closed / %02d : %02d", hours, left % 60);

			return text;
		}

		// The owner's records as the shop's sales window keeps them; the
		// total is what the records add up to.
		inline personal_shop::Sales sales(const std::vector<mini_room::MerchantSale> &sold, int32_t net)
		{
			personal_shop::Sales sales;

			for (const mini_room::MerchantSale &sale : sold)
			{
				sales.sold.push_back({ sale.itemid, sale.quantity, sale.buyer });
				sales.total += sale.meso;
			}

			sales.received = net;

			return sales;
		}
	}
}
