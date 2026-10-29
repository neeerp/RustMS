#pragma once

#include <cstdint>
#include <map>
#include <optional>
#include <string>
#include <vector>

namespace ms
{
	// What the character info window lists beside the character (v83 CUIUserInfo): the equipment in the
	// item panel, the wish list, and the medal titles on the collection page.
	namespace char_info
	{
		// The item panel's list (FUN_00903e24), from a look's equipment by body part: for each part from 1 to
		// 0x33, less 0xe and the mount's 0x12 to 0x14, the cash weapon (by the weapon's), the equipment a cash
		// item covers, then the one shown, less pet equipment.
		inline std::vector<int32_t> worn_items(const std::map<int8_t, int32_t> &equips,
											   const std::map<int8_t, int32_t> &masked)
		{
			auto at = [](const std::map<int8_t, int32_t> &parts, int8_t part)
			{
				auto found = parts.find(part);

				return found == parts.end() ? 0 : found->second;
			};

			std::vector<int32_t> items;

			for (int8_t part = 1; part < 0x34; part++)
			{
				if (part == 0xe || (part >= 0x12 && part <= 0x14))
					continue;

				if (int32_t cash = at(masked, -111); part == 0xb && cash / 100000 == 17)
					items.push_back(cash);

				if (int32_t covered = at(masked, part); covered > 0)
					items.push_back(covered);

				if (int32_t shown = at(equips, part); shown > 0 && shown / 10000 != 181)
					items.push_back(shown);
			}

			return items;
		}

		// A wish, as the Cash Shop sells it.
		struct Wish
		{
			int32_t sn;
			int32_t item_id;
			int16_t count;
		};

		// The wish list's rows (FUN_009041c8): the SNs from last to first, less those the Cash Shop doesn't
		// sell or that are no item. `find` gives a commodity's item and count by its SN.
		template <typename Find>
		std::vector<Wish> wishes(const std::vector<int32_t> &sns, Find find)
		{
			std::vector<Wish> rows;

			for (auto sn = sns.rbegin(); sn != sns.rend(); ++sn)
				if (std::optional<Wish> wish = find(*sn); wish && wish->item_id != 0)
					rows.push_back(*wish);

			return rows;
		}

		// A row's name: the item's, with its count after it when the Cash Shop sells more than one.
		inline std::string wish_name(const std::string &name, int16_t count)
		{
			return count > 1 ? name + " (" + std::to_string(count) + ")" : name;
		}

		// The rows of a list that can scroll, which shows `rows` at a time: the first can be from 0 to this.
		inline int16_t last_top(size_t count, int16_t rows)
		{
			return count > static_cast<size_t>(rows) ? static_cast<int16_t>(count - rows) : 0;
		}

		// The medal titles shown (0x902ae0): the medals of the quests from `top`, four places of the list, of
		// which a quest with no medal (`medal` 0) takes none.
		template <typename Medal>
		std::vector<int32_t> medal_rows(const std::vector<int16_t> &quests, size_t top, Medal medal)
		{
			constexpr size_t ROWS = 4;
			std::vector<int32_t> shown;

			for (size_t i = top; i < quests.size() && i < top + ROWS; i++)
				if (int32_t item = medal(quests[i]); item != 0)
					shown.push_back(item);

			return shown;
		}

		// The level's icon (0x685c0f): `MonsterBook/icon/<level - 1>`, if there is one.
		inline std::optional<int32_t> level_icon(int32_t level, size_t icons)
		{
			if (level <= 0 || static_cast<size_t>(level) > icons)
				return std::nullopt;

			return level - 1;
		}
	}
}
