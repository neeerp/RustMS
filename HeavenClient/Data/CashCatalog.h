//////////////////////////////////////////////////////////////////////////////////
//	This file is part of the continued Journey MMORPG client					//
//	Copyright (C) 2015-2019  Daniel Allendorf, Ryan Payton						//
//																				//
//	This program is free software: you can redistribute it and/or modify		//
//	it under the terms of the GNU Affero General Public License as published by	//
//	the Free Software Foundation, either version 3 of the License, or			//
//	(at your option) any later version.											//
//																				//
//	This program is distributed in the hope that it will be useful,				//
//	but WITHOUT ANY WARRANTY; without even the implied warranty of				//
//	MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the				//
//	GNU Affero General Public License for more details.							//
//																				//
//	You should have received a copy of the GNU Affero General Public License	//
//	along with this program.  If not, see <https://www.gnu.org/licenses/>.		//
//////////////////////////////////////////////////////////////////////////////////
#pragma once

#include <algorithm>
#include <array>
#include <cstdint>
#include <string>
#include <utility>
#include <vector>

namespace ms
{
	// The commodities of Etc.nx Commodity.img, and how the v83 Cash Shop sorts
	// them into tabs, sub-tabs and pages.
	namespace cash_catalog
	{
		// The label drawn over an item's icon (CashShop.img/CSEffect).
		enum Label : int8_t
		{
			NONE = -1,
			NEW,
			SALE,
			HOT,
			EVENT
		};

		struct Commodity
		{
			int32_t sn;
			int32_t item_id;
			int32_t price;
			int16_t count;
			int8_t gender;
			Label label;
			int32_t priority;
			// Days the item lasts; 0 is forever.
			int32_t period = 0;
			bool on_sale = true;
		};

		// Tabs in the order they're drawn. MAIN shows the how-to pages
		// (category 8); the others hold the commodities whose SN starts
		// with their number. WISH_LIST shows the player's wish list.
		enum Tab : uint8_t
		{
			MAIN,
			EVENT_TAB,
			EQUIP,
			USE,
			SETUP,
			ETC,
			PET,
			PACKAGE,
			WISH_LIST,
			NUM_TABS
		};

		constexpr int32_t MAIN_CATEGORY = 8;
		constexpr size_t PAGE_SIZE = 10;
		constexpr size_t WISH_LIST_SIZE = 10;

		inline int32_t category(Tab tab)
		{
			return tab == MAIN ? MAIN_CATEGORY : tab;
		}

		// A commodity's SN holds its tab and sub-tab: 50200003 is in tab 5
		// (ETC), sub-tab 2 (Game).
		inline int32_t category(int32_t sn)
		{
			return sn / 10000000;
		}

		inline int32_t subcategory(int32_t sn)
		{
			return sn / 100000 % 100;
		}

		// Commodity.img's Class names the label. Unlabelled items in
		// Event > New are shown as NEW, as in period footage.
		inline Label label(int32_t item_class, int32_t sn)
		{
			if (item_class >= NEW && item_class <= EVENT)
				return static_cast<Label>(item_class);

			if (category(sn) == 1 && subcategory(sn) == 0)
				return NEW;

			return NONE;
		}

		// Gender 0 is male, 1 female; anything else is for both.
		inline bool for_gender(const Commodity& commodity, bool female)
		{
			if (commodity.gender == 0)
				return !female;

			if (commodity.gender == 1)
				return female;

			return true;
		}

		// One sub-tab's commodities for the player, highest Priority first,
		// then by SN. Labelled items have the highest priorities, so period
		// footage shows a sub-tab's first page all HOT.
		inline std::vector<Commodity> list(const std::vector<Commodity>& all, int32_t cat, int32_t sub, bool female)
		{
			std::vector<Commodity> found;

			for (const Commodity& commodity : all)
				if (category(commodity.sn) == cat && subcategory(commodity.sn) == sub && for_gender(commodity, female))
					found.push_back(commodity);

			std::stable_sort(found.begin(), found.end(),
				[](const Commodity& a, const Commodity& b)
				{
					if (a.priority != b.priority)
						return a.priority > b.priority;

					return a.sn < b.sn;
				}
			);

			return found;
		}

		// Commodities 80000000 to 89999999 cost mesos, not NX (v83 46e5c5).
		inline bool for_mesos(int32_t sn)
		{
			return sn >= 80000000 && sn < 90000000;
		}

		// Where a mesos commodity is listed: a sub-tab and a roll for its
		// place there.
		struct Placement
		{
			Commodity commodity;
			int32_t cat;
			int32_t sub;
			uint32_t roll;
		};

		// v83 (471f37) has no tab for the mesos commodities: when the shop
		// opens, each one on sale goes into a random sub-tab that has items.
		template<class Roll>
		std::vector<Placement> scatter(const std::vector<Commodity>& on_sale, Roll roll)
		{
			std::vector<std::pair<int32_t, int32_t>> groups;

			for (const Commodity& commodity : on_sale)
			{
				std::pair<int32_t, int32_t> group = { category(commodity.sn), subcategory(commodity.sn) };

				if (group.first >= 1 && group.first < MAIN_CATEGORY
					&& std::find(groups.begin(), groups.end(), group) == groups.end())
					groups.push_back(group);
			}

			std::vector<Placement> placed;

			if (groups.empty())
				return placed;

			for (const Commodity& commodity : on_sale)
			{
				if (!for_mesos(commodity.sn))
					continue;

				std::pair<int32_t, int32_t> group = groups[roll() % groups.size()];
				placed.push_back({ commodity, group.first, group.second, static_cast<uint32_t>(roll()) });
			}

			return placed;
		}

		// Puts the mesos commodities scattered into this sub-tab among its
		// list, each at its roll modulo the list's size, as v83 does.
		inline void place(std::vector<Commodity>& found, const std::vector<Placement>& placed, int32_t cat, int32_t sub, bool female)
		{
			for (const Placement& placement : placed)
			{
				if (placement.cat != cat || placement.sub != sub || !for_gender(placement.commodity, female))
					continue;

				size_t index = found.empty() ? 0 : placement.roll % found.size();
				found.insert(found.begin() + index, placement.commodity);
			}
		}

		// The commodities with these SNs, in order, skipping unknown ones.
		inline std::vector<Commodity> find(const std::vector<Commodity>& all, const std::vector<int32_t>& sns)
		{
			std::vector<Commodity> found;

			for (int32_t sn : sns)
			{
				auto iter = std::find_if(all.begin(), all.end(), [sn](const Commodity& commodity) { return commodity.sn == sn; });

				if (sn != 0 && iter != all.end())
					found.push_back(*iter);
			}

			return found;
		}

		// What a package's content costs on its own (FUN_00a25ae9): the
		// dearest commodity with its item, count and period.
		inline int32_t regular_price(const std::vector<Commodity>& all, const Commodity& content)
		{
			int32_t price = content.price;

			for (const Commodity& commodity : all)
				if (commodity.item_id == content.item_id && commodity.count == content.count
					&& commodity.period == content.period)
					price = std::max(price, commodity.price);

			return price;
		}

		inline size_t page_count(size_t items)
		{
			return std::max<size_t>(1, (items + PAGE_SIZE - 1) / PAGE_SIZE);
		}

		// The wish list with sn added at the end, unless it's there already
		// or the list is full.
		inline std::vector<int32_t> wish(std::vector<int32_t> sns, int32_t sn)
		{
			sns.erase(std::remove(sns.begin(), sns.end(), 0), sns.end());

			if (sns.size() < WISH_LIST_SIZE && std::find(sns.begin(), sns.end(), sn) == sns.end())
				sns.push_back(sn);

			return sns;
		}

		// Why v83 (FUN_00470d7d) won't add sn to the wish list, or nullptr when it will: a full list
		// first, then one that holds the same item, count and period under any SN (FUN_004729fd).
		inline const char* wish_refusal(const std::vector<Commodity>& all, const std::vector<int32_t>& sns, int32_t sn)
		{
			if (std::count_if(sns.begin(), sns.end(), [](int32_t w) { return w != 0; }) >= WISH_LIST_SIZE)
				return "Your wish list is full.";

			auto wished = std::find_if(all.begin(), all.end(), [sn](const Commodity& c) { return c.sn == sn; });

			for (int32_t listed : sns)
			{
				if (listed == 0)
					continue;

				if (listed == sn)
					return "You already have this item on your wish list.";

				auto other = std::find_if(all.begin(), all.end(), [listed](const Commodity& c) { return c.sn == listed; });

				if (wished != all.end() && other != all.end() && other->item_id == wished->item_id
					&& other->count == wished->count && other->period == wished->period)
					return "You already have this item on your wish list.";
			}

			return nullptr;
		}

		// Whether v83 sends a wish it doesn't refuse: not for an SN missing from the catalog
		// (FUN_00471be9), nor for a free one.
		inline bool wish_sent(const std::vector<Commodity>& all, int32_t sn)
		{
			auto wished = std::find_if(all.begin(), all.end(), [sn](const Commodity& c) { return c.sn == sn; });

			return wished != all.end() && wished->price != 0;
		}

		// The wish list without sn.
		inline std::vector<int32_t> unwish(std::vector<int32_t> sns, int32_t sn)
		{
			sns.erase(std::remove(sns.begin(), sns.end(), sn), sns.end());
			sns.erase(std::remove(sns.begin(), sns.end(), 0), sns.end());

			return sns;
		}

		// Why v83 (FUN_004bb884) won't buy a Best Item on a double-click, or nullptr when it will:
		// megaphones (507) need level 11, store permits (514) and Hired Merchants (503) level 16,
		// and Teleport Rocks (504) level 7. It also keeps the shops from accounts registered
		// less than 14 days ago (0xea5), a date HC doesn't get.
		inline const char* best_item_refusal(int32_t item_id, int32_t level)
		{
			int32_t kind = item_id / 10000;

			if (kind == 507 && level <= 10)
				return "This megaphone is only available for \\ncharacters that are over Level 10.";

			if ((kind == 514 || kind == 503) && level <= 15)
				return "Hired Merchant's Shop is only available for \\ncharacters that are over Level 15.";

			if (kind == 504 && level <= 6)
				return "This teleport rock is only available for \\ncharacters that are over level 6.";

			return nullptr;
		}

		// What a purchase is paid with, as the server numbers it.
		enum CashType : int32_t
		{
			CREDIT = 1,
			MAPLE_POINTS = 2,
			PREPAID = 4
		};

		// The BUY dialog's options, top to bottom.
		constexpr size_t NUM_PAYMENTS = 3;
		constexpr CashType PAYMENTS[NUM_PAYMENTS] = { MAPLE_POINTS, PREPAID, CREDIT };

		// The options the BUY dialog offers, in its order: each balance
		// that covers the price, and Maple Points alone for a free item
		// (v83 46dbe5).
		inline std::array<bool, NUM_PAYMENTS> payment_options(int32_t price, int32_t credit, int32_t points, int32_t prepaid)
		{
			if (price <= 0)
				return { true, false, false };

			return { points >= price, prepaid >= price, credit >= price };
		}

		// The options Buy Avatar's dialog offers for its total (v83 4696b1):
		// each balance that covers it, but not NX Credit when any of the
		// items is free.
		inline std::array<bool, NUM_PAYMENTS> avatar_payment_options(int32_t total, bool any_free, int32_t credit, int32_t points, int32_t prepaid)
		{
			return { total <= points, total <= prepaid, !any_free && total <= credit };
		}

		// StringPool 0x1fb then 0x1fc, as v83 joins them.
		inline std::string avatar_question(int32_t total)
		{
			return "Total " + std::to_string(total) + " NX.Make purchase?";
		}

		// Whether the mannequin tries the item on (v83 4ac3de, 460adc and
		// 4606a0): an equip, not 16xxxxx, unless its id's thousands digit
		// is the other gender's (0 male, 1 female; any other is for both).
		inline bool fits_mannequin(int32_t item_id, bool female)
		{
			if (item_id / 1000000 != 1 || item_id / 100000 == 16)
				return false;

			int32_t gender = item_id / 1000 % 10;

			return gender > 1 || gender == (female ? 1 : 0);
		}

		// The option checked when the BUY dialog opens: NX Prepaid, as in
		// period footage, unless only another offered balance pays.
		inline size_t default_payment(const std::array<bool, NUM_PAYMENTS>& offered)
		{
			for (size_t option : { 1, 2, 0 })
				if (offered[option])
					return option;

			return 1;
		}

		// Items only NX Prepaid pays for: Name Change, World Transfer,
		// 520000x, 549xxxx, 5221001 and 5222000 (v83 46dc4d).
		inline bool prepaid_only(int32_t item_id)
		{
			return item_id == 5400000 || item_id == 5401000 || item_id / 10 == 520000 || item_id / 10000 == 549
				|| item_id == 5221001 || item_id == 5222000;
		}

		// The birthday code that confirms a gift or a Name Change, typed
		// as the date YYYYMMDD.
		constexpr size_t BIRTHDAY_LIMIT = 8;

		// The birthday code as the number YYYYMMDD, or 0 if it isn't one.
		inline int32_t birthday_code(const std::string& text)
		{
			if (text.size() != BIRTHDAY_LIMIT)
				return 0;

			for (char c : text)
				if (c < '0' || c > '9')
					return 0;

			return std::stoi(text);
		}

		// Whether the code is a real date, as the server requires before
		// it acts on one (`net::cash_shop::birthday_code`).
		inline bool birthday_date(int32_t code)
		{
			int32_t year = code / 10000, month = code / 100 % 100, day = code % 100;
			bool leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
			constexpr int32_t DAYS[12] = { 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31 };

			if (year < 1900 || year > 9999 || month < 1 || month > 12)
				return false;

			return day >= 1 && day <= DAYS[month - 1] + (month == 2 && leap ? 1 : 0);
		}

		inline bool affordable(int32_t price, int32_t credit, int32_t points, int32_t prepaid)
		{
			return credit >= price || points >= price || prepaid >= price;
		}

		// Slot expansions (v83 46c036, 46c55b, 470653). The Cash Inventory's
		// buttons add four slots for 4,000 NX, the commodities their item's
		// info/delta. Bags hold at most 96 slots, the storage 48.
		constexpr int32_t EXPANSION_SLOTS = 4;
		constexpr int32_t EXPANSION_PRICE = 4000;
		constexpr int32_t MAX_BAG_SLOTS = 96;
		constexpr int32_t MAX_STORAGE_SLOTS = 48;

		// The Extra Character Slot Coupon (5430xxx) adds a slot to the
		// account at once, up to 15 (v83 46c6f8 and 8f1b3d).
		constexpr int16_t MAX_CHARACTER_SLOTS = 15;

		inline bool is_character_slot(int32_t item_id)
		{
			return item_id / 1000 == 5430;
		}

		// v83 47ac0a takes a new count only above the last and at most 15.
		inline bool added_character_slot(int16_t before, int16_t after)
		{
			return after > before && after <= MAX_CHARACTER_SLOTS;
		}

		// A commodity SET_CASH_SHOP changes, or adds if Commodity.img lacks
		// its SN: only the fields `flags` names are sent (v83 a25db4, report
		// b179).
		struct Change
		{
			enum Field : uint32_t
			{
				ITEM_ID = 0x1,
				COUNT = 0x2,
				PRICE = 0x4,
				BONUS = 0x8,
				PRIORITY = 0x10,
				PERIOD = 0x20,
				MAPLE_POINT = 0x40,
				MESO = 0x80,
				PREMIUM = 0x100,
				GENDER = 0x200,
				ON_SALE = 0x400,
				CLASS = 0x800,
				LIMIT = 0x1000,
				PB_CASH = 0x2000,
				PB_POINT = 0x4000,
				PB_GIFT = 0x8000,
				PACKAGE = 0x10000
			};

			int32_t sn;
			uint32_t flags;
			int32_t item_id = 0;
			int16_t count = 0;
			int32_t priority = 0;
			int32_t price = 0;
			int32_t period = 0;
			int8_t gender = 0;
			bool on_sale = false;
			int32_t item_class = -1;
		};

		// The commodity `change` makes of the one with its SN in `all`, or
		// of a new one, which has only what is sent (a25f19).
		inline void apply_change(std::vector<Commodity>& all, const Change& change)
		{
			auto found = std::find_if(all.begin(), all.end(),
				[&](const Commodity& commodity) { return commodity.sn == change.sn; });

			if (found == all.end())
			{
				Commodity added = { change.sn, 0, 0, 0, 0, NONE, 0, 0, false };
				all.push_back(added);
				found = all.end() - 1;
			}

			Commodity& commodity = *found;

			if (change.flags & Change::ITEM_ID)
				commodity.item_id = change.item_id;

			if (change.flags & Change::COUNT)
				commodity.count = change.count;

			if (change.flags & Change::PRIORITY)
				commodity.priority = change.priority;

			if (change.flags & Change::PRICE)
				commodity.price = change.price;

			if (change.flags & Change::PERIOD)
				commodity.period = change.period;

			if (change.flags & Change::GENDER)
				commodity.gender = change.gender;

			if (change.flags & Change::ON_SALE)
				commodity.on_sale = change.on_sale;

			if (change.flags & Change::CLASS)
				commodity.label = label(change.item_class, change.sn);
		}

		// The commodities on sale, in the order of `all`.
		inline std::vector<Commodity> on_sale(const std::vector<Commodity>& all)
		{
			std::vector<Commodity> found;

			for (const Commodity& commodity : all)
				if (commodity.on_sale)
					found.push_back(commodity);

			return found;
		}

		// A slot extender (555xxxx) keeps an extra equip slot open for its
		// `addDay`; v83 buys one with its own question and request, whatever
		// its SN (4731a9, 46c988).
		inline bool is_slot_extender(int32_t item_id)
		{
			return item_id / 10000 == 555;
		}

		// StringPool 0x1458, as v83 words it.
		inline std::string slot_extender_question(const std::string& name, int32_t price, int32_t days)
		{
			return "If you purchase a " + name + "(" + std::to_string(price) + "cash), you can equip \\n the item for "
				+ std::to_string(days) + " additional days. \\n Refunds are not availabe after the purchase. \\n "
				"Do you still wish to purchase it?";
		}

		// The slots a coming job advancement adds to a bag (1 Equip to 4
		// Etc) that adding `added` now would leave no room for, or 0 (v83
		// 46c38f). v83 asks before such a purchase.
		inline int32_t advancement_slots(int32_t job, int32_t bag, int32_t slots, int32_t added)
		{
			constexpr int32_t EQUIP_BAG = 1, USE_BAG = 2, ETC_BAG = 4;

			int32_t branch = job % 1000 / 100;
			int32_t after = slots + added;

			if (branch == 0)
			{
				if (bag == EQUIP_BAG)
					return after > MAX_BAG_SLOTS - 4 ? 4 : 0;

				return (bag == USE_BAG || bag == ETC_BAG) && after > MAX_BAG_SLOTS - 8 ? 8 : 0;
			}

			// Only first and second jobs are asked.
			if (job % 10 != 0)
				return 0;

			bool widens = false;

			switch (branch)
			{
			case 1:
			case 5:
				widens = bag == USE_BAG || bag == ETC_BAG;
				break;
			case 2:
			case 3:
				widens = bag == ETC_BAG;
				break;
			case 4:
				widens = bag == USE_BAG;
				break;
			}

			return widens && after > MAX_BAG_SLOTS - 4 ? 4 : 0;
		}
	}
}
