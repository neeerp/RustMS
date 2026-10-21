#pragma once

#include <algorithm>
#include <cstdint>
#include <functional>
#include <iterator>
#include <string>
#include <utility>
#include <vector>

namespace ms
{
	// The Item Maker (skill 1007, window type 0x17): what the v83 client
	// checks and shows (FUN_00822abb, FUN_008232bf, FUN_008234a3,
	// FUN_00824e8d; reports c006 and c006b).
	namespace maker
	{
		// The window's combo box data for its fixed entries.
		constexpr int32_t GEMS = 425;
		constexpr int32_t CRYSTAL = 426;
		constexpr int32_t USE = 200;
		constexpr int32_t SETUP = 300;
		constexpr int32_t ETC = 400;
		constexpr int32_t DISASSEMBLE = 998;
		constexpr int32_t RANDOM = 999;

		// A crystal takes 100 of a drop.
		constexpr int32_t CRYSTAL_DROPS = 100;
		// The gauge runs 5100 ms before the request goes (FUN_00824a06).
		constexpr int32_t GAUGE_MS = 5100;

		struct Recipe
		{
			int32_t item_id = 0;
			int32_t group = 0;
			int32_t count = 1;
			int32_t meso = 0;
			std::vector<std::pair<int32_t, int32_t>> ingredients;
			// The stimulator equipment takes, 0 for none.
			int32_t catalyst = 0;
			int32_t req_level = 0;
			int32_t req_skill_level = 0;
			int32_t req_item = 0;
			int32_t req_equip = 0;
			// Each quest and the state it must be in: 1 started, 2 completed, 3 either.
			std::vector<std::pair<int32_t, int32_t>> req_quests;
			// The gem slots the window gives it (tuc).
			int32_t gem_slots = 0;
		};

		// The Maker skill of a job: base * 10000 + 1007, Evan's base being 2001.
		inline int32_t skill_id(int16_t job)
		{
			int32_t base = job / 1000 * 1000;

			if (job / 100 == 22 || job == 2001)
				base = 2001;

			return base * 10000 + 1007;
		}

		// Whether a skill is a Maker, which opens the window and sends
		// nothing (FUN_00966f7a).
		inline bool is_maker(int32_t skill_id)
		{
			return skill_id == 1007 || skill_id == 10001007 || skill_id == 20001007 || skill_id == 20011007;
		}

		// The recipe groups a job sees: 0 for everyone, then one bit for its
		// class (job % 1000 / 100), or all five for a GM's.
		inline std::vector<int32_t> groups(int16_t job)
		{
			std::vector<int32_t> seen = { 0 };
			int32_t job_class = job % 1000 / 100;

			if (job_class == 9)
			{
				for (int32_t i = 0; i < 5; i++)
					seen.push_back(1 << i);
			}
			else if (job_class != 0)
			{
				seen.push_back(1 << (job_class - 1));
			}

			return seen;
		}

		// The category a recipe is listed under: its item id / 10000, with
		// Use, Setup and Etc items each under one, but the gems apart.
		inline int32_t category(int32_t item_id)
		{
			int32_t key = item_id / 10000;

			if (key != GEMS)
			{
				int32_t kind = key / 100 * 100;

				if (kind == USE || kind == SETUP || kind == ETC)
					return kind;
			}

			return key;
		}

		// Whether a quest in `state` (0 not started, 1 started, 2 completed)
		// is as a recipe wants it.
		inline bool quest_met(int32_t want, int32_t state)
		{
			if (want == 3)
				return state == 1 || state == 2;

			return state == want;
		}

		// What the window knows of its player when it lists recipes.
		struct Maker
		{
			int32_t level = 0;
			int32_t maker_level = 0;
			std::function<bool(int32_t)> holds;
			std::function<bool(int32_t)> wears;
			// A quest's state: 0 not started, 1 started, 2 completed.
			std::function<int32_t(int32_t)> quest_state;
		};

		// Whether a recipe is listed (FUN_00822abb): the level, the Maker's
		// level, the item held, the equipment worn and the quests.
		inline bool listed(const Recipe& recipe, const Maker& maker)
		{
			if (maker.level < recipe.req_level || maker.maker_level < recipe.req_skill_level)
				return false;

			if (recipe.req_item != 0 && !maker.holds(recipe.req_item))
				return false;

			if (recipe.req_equip != 0 && !maker.wears(recipe.req_equip))
				return false;

			for (auto [quest, want] : recipe.req_quests)
				if (!quest_met(want, maker.quest_state(quest)))
					return false;

			return true;
		}

		// The first combo's entries, by their data (FUN_008234a3): its title,
		// disassembly, the crystal and the gems, then each kind of equipment
		// the recipes make in their order, Use, Setup and Etc, and last the
		// random make. The gems, Use, Setup and Etc only with a recipe.
		inline std::vector<int32_t> categories(const std::vector<const Recipe*>& recipes)
		{
			std::vector<int32_t> keys = { 0, DISASSEMBLE, CRYSTAL };
			std::vector<int32_t> found;

			for (const Recipe* recipe : recipes)
			{
				int32_t key = category(recipe->item_id);

				if (std::find(found.begin(), found.end(), key) == found.end())
					found.push_back(key);
			}

			auto has = [&](int32_t key)
			{
				return std::find(found.begin(), found.end(), key) != found.end();
			};

			if (has(GEMS))
				keys.push_back(GEMS);

			for (int32_t key : found)
				if (key < USE)
					keys.push_back(key);

			for (int32_t key : { USE, SETUP, ETC })
				if (has(key))
					keys.push_back(key);

			keys.push_back(RANDOM);

			return keys;
		}

		// An equipment category's name, from the item id of its first recipe
		// (FUN_00823039: the weapon names of FUN_005c99fc, else the armour's of
		// FUN_005c9e61, as the string pool has them: 0x259-0x268, 0x269-0x277).
		// U: whether the crossbow's and claw's have the leading space.
		inline std::string equip_category(int32_t item_id)
		{
			static const char* const ARMOUR[] = {
				"HAT", "FACE ACCESSORY", "EYE ACCESSORY", "EARRING", "TOP", "OVERALL", "BOTTOM", "SHOES",
				"GLOVE", "SHIELD", "CAPE", "RING", "PENDANT", "BELT", "MEDAL"
			};
			static const char* const WEAPONS[] = {
				" ONE-HANDED SWORD", " ONE-HANDED AXE", " ONE_HANDED MACE", " DAGGER", "", "", "", " WAND", " STAFF",
				"", " TWO-HANDED SWORD", " TWO-HANDED AXE", " TWO-HANDED MACE", " SPEAR", " POLE ARM", " BOW",
				" CROSSBOW", " CLAW", "KNUCKLE", "GUN"
			};
			int32_t key = item_id / 10000;

			if (key >= 100 && key < 115)
				return ARMOUR[key - 100];

			if (key >= 130 && key < 150)
				return WEAPONS[key - 130];

			return {};
		}

		// The recipes listed under a category, in their order.
		inline std::vector<const Recipe*> under(int32_t key, const std::vector<const Recipe*>& recipes)
		{
			std::vector<const Recipe*> listed;

			for (const Recipe* recipe : recipes)
				if (category(recipe->item_id) == key)
					listed.push_back(recipe);

			return listed;
		}

		// The monster crystal made from a drop of `level`, from each crystal's
		// levels (lvMin, lvMax), or 0 for none.
		struct CrystalLevels
		{
			int32_t crystal;
			int32_t low;
			int32_t high;
		};

		inline int32_t crystal_for(int32_t level, const std::vector<CrystalLevels>& crystals)
		{
			if (level <= 0)
				return 0;

			for (const CrystalLevels& c : crystals)
				if (level >= c.low && level <= c.high)
					return c.crystal;

			return 0;
		}

		// The fee the window shows (FUN_00824e8d): each part adds a tenth of
		// the recipe's mesos, one for the recipe, five for a stimulator, and
		// three for each gem plus three for each grade of it. Floored to a
		// thousand.
		inline int32_t fee(int32_t meso, bool stimulator, const std::vector<int32_t>& gems)
		{
			int64_t parts = (stimulator ? 5 : 0) + 1;

			for (int32_t gem : gems)
				parts += 3 * (1 + gem % 10);

			int64_t total = meso + parts * (meso / 10);

			if (total <= 0)
				return 0;

			return static_cast<int32_t>(total / 1000 * 1000);
		}

		// Whether a recipe's product is a weapon, which FUN_005c99fc names.
		inline bool is_weapon(int32_t item_id)
		{
			return item_id / 10000 >= 130 && !equip_category(item_id).empty();
		}

		// A gem's kind (FUN_0082637e): its id's hundreds, 0 for weapon
		// attack and 1 for magic attack.
		inline int32_t gem_kind(int32_t gem)
		{
			return gem % 10000 / 100;
		}

		// Why a gem can't go in beside `gems` in `slots` slots, in the
		// order the window asks (FUN_0082637e, StringPool 0x2e6-0x2e9), or
		// nothing when it can. `weapon` is whether the product is one.
		inline std::string gem_refusal(const std::vector<int32_t>& gems, int32_t slots, int32_t gem, bool weapon)
		{
			if (static_cast<int32_t>(gems.size()) >= slots)
				return "You cannot apply the Strengthening Gem anymore.";

			if (gem / 10000 != GEMS)
				return "You can only apply the Strengthening Gem.";

			for (int32_t other : gems)
				if (gem_kind(other) == gem_kind(gem))
					return "You cannot apply more than 1 of the same Strengthening Gems.";

			if (gem_kind(gem) <= 1 && !weapon)
				return "You can only use this Strengthening Gem on weapon items.";

			return {};
		}

		// Why a stimulator can't go in for a recipe of `catalyst`, or
		// nothing when it can (FUN_008265a1, StringPool 0x2ea-0x2ec).
		inline std::string stimulator_refusal(bool applied, int32_t item_id, int32_t catalyst)
		{
			if (applied)
				return "You can only apply 1 Stimulator.";

			if (item_id / 1000 != 4130)
				return "You can only apply the Stimulator.";

			if (item_id != catalyst)
				return "You cannot apply this Stimulator.";

			return {};
		}

		// How far an equip's stats are above its own (FUN_005ca1b8): -1 when
		// below them, else 0 to 3. Both lists hold STR, DEX, INT, LUK, HP, MP,
		// attack, magic attack, defense, magic defense, accuracy, avoid,
		// speed and jump; HP and MP count a tenth.
		inline int32_t stat_grade(const std::vector<int32_t>& stats, const std::vector<int32_t>& own)
		{
			int32_t score = 0;

			for (size_t i = 0; i < stats.size() && i < own.size(); i++)
			{
				int32_t above = stats[i] - own[i];
				score += i == 4 || i == 5 ? above / 10 : above;
			}

			if (score < 0)
				return -1;

			if (score <= 5)
				return 0;

			if (score <= 22)
				return 1;

			return score <= 39 ? 2 : 3;
		}

		// The fee to take apart what a recipe of `meso` makes, of `grade`
		// (FUN_00824f9d, FUN_005ca659): a tenth of it, and half that again
		// for each grade above -1, rounded down to 1000.
		inline int32_t disassembly_fee(int32_t meso, int32_t grade)
		{
			int64_t fee = static_cast<int64_t>(meso / 10) * (150 + 50 * grade) / 100;

			return fee <= 0 ? 0 : static_cast<int32_t>(fee / 1000 * 1000);
		}

		// Whether what a recipe makes may be taken apart (FUN_0082317f): it is
		// equipment.
		inline bool disassemblable(const Recipe& recipe)
		{
			static const int32_t GROUPS[] = { 0, 1, 2, 4, 8, 16 };

			return recipe.item_id / 1000000 == 1 &&
				   std::find(std::begin(GROUPS), std::end(GROUPS), recipe.group) != std::end(GROUPS);
		}

		// The count under an ingredient's icon (formats at 0xb38d00 and 0xb39390).
		inline std::string count_text(int32_t owned, int32_t needed)
		{
			return std::to_string(owned) + "/" + std::to_string(needed);
		}

		// The Yes/No before making (StringPool 0x2f4, 0x2f5, 0x2f7).
		inline std::string confirm_text(int32_t mode, const std::string& name)
		{
			switch (mode)
			{
				case 2:
					return "Would you like to make ???";
				case 3:
					return "Would you like to make a Monster Crystal?";
				case 4:
					return "Would you like to disassemble?";
				default:
					return name + ": Would you like to make this?";
			}
		}

		// No room in the tab `type`, 1 Equip to 4 Etc (FUN_00827559).
		inline std::string no_room_text(int32_t type)
		{
			static const char* const TABS[] = { "Eqp", "Use", "Setup", "Etc" };

			return std::string(TABS[std::clamp(type, 1, 4) - 1]) + " You don't have enough room in your Inventory.";
		}

		// The dialog MAKER_RESULT ends with (FUN_00827264, FUN_00827559): for
		// a made item its count and name, else nothing but the code. Empty
		// for none.
		inline std::string result_text(int32_t result, int32_t count, const std::string& name)
		{
			switch (result)
			{
				case 0:
					if (name.empty())
						return {};

					return std::to_string(count) + " of " + name + "  have been produced.";
				case 1:
					return "The item has been destroyed.";
				case 2:
					return "Production failed due to an unknown error.";
				case 4:
				case 5:
				case 6:
				case 7:
					return no_room_text(result - 3);
				default:
					return {};
			}
		}

		// The chat lines MAKER_RESULT writes (FUN_0095dad3, message type 7):
		// the counts always follow the names.
		inline std::string gained_line(const std::string& tab, const std::string& name, int32_t count)
		{
			return "You have gained items in the " + tab + " tab (" + name + " " + std::to_string(count) + ")";
		}

		inline std::string lost_line(const std::string& name, int32_t count)
		{
			return "You have lost items (" + name + " " + std::to_string(count) + ")";
		}

		inline std::string lost_item_line(const std::string& name)
		{
			return "You have lost an item (" + name + ")";
		}

		// The dialog after a disassembly (StringPool 0x2f0).
		inline std::string disassembled_text(const std::string& name)
		{
			return "You have successfully disassembled the " + name + ".";
		}

		inline std::string meso_line(int32_t meso)
		{
			return "You have lost mesos. (" + std::to_string(-meso) + ")";
		}
	}
}
