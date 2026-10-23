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
#include <cstdint>
#include <string>
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
		// first, then one that has it already.
		inline const char* wish_refusal(const std::vector<int32_t>& sns, int32_t sn)
		{
			if (std::count_if(sns.begin(), sns.end(), [](int32_t w) { return w != 0; }) >= WISH_LIST_SIZE)
				return "Your wish list is full.";

			if (std::find(sns.begin(), sns.end(), sn) != sns.end())
				return "You already have this item on your wish list.";

			return nullptr;
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

		// The option checked when the BUY dialog opens: NX Prepaid, as in
		// period footage, unless only another balance covers the price.
		inline size_t default_payment(int32_t price, int32_t credit, int32_t points, int32_t prepaid)
		{
			const int32_t balances[NUM_PAYMENTS] = { points, prepaid, credit };

			for (size_t option : { 1, 2, 0 })
				if (balances[option] >= price)
					return option;

			return 1;
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
