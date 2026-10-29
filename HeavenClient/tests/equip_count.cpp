#include "../Character/Inventory/EquipCount.h"
#include <iostream>
#include <stdexcept>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

int main()
{
	try
	{
		using namespace ms::equip_count;

		Requirements beginners;
		beginners.job = -1;
		Requirements warriors;
		warriors.job = 1 | 8;
		warriors.level = 10;

		Wearer wearer;
		wearer.level = 1;
		require(met_by(beginners, wearer, {}), "a beginner wears a beginner's item");
		require(!met_by(warriors, wearer, {}), "but not a warrior's");
		wearer.job = 2110;
		wearer.level = 10;
		require(!met_by(beginners, wearer, {}) && met_by(warriors, wearer, {}), "an Aran wears a warrior's item");
		wearer.level = 9;
		require(!met_by(warriors, wearer, {}), "short of its level");
		wearer.job = 900;
		require(met_by(warriors, wearer, {}), "GMs wear anything");
		Requirements famous;
		famous.fame = 1;
		wearer.job = 0;
		wearer.fame = -3;
		require(!met_by(famous, wearer, {}), "short of fame");

		// The sword needs 15 STR and a warrior, the shield 10 STR.
		Requirements sword_needs;
		sword_needs.job = 1;
		sword_needs.stats = { 15, 0, 0, 0 };
		Requirements shield_needs;
		shield_needs.stats = { 10, 0, 0, 0 };
		auto worn = [](int16_t slot, int32_t id, int32_t str, Requirements needs) {
			Worn item;
			item.slot = slot;
			item.item_id = id;
			item.bonus = { str, 0, 0, 0 };
			item.requirements = needs;
			return item;
		};

		wearer = {};
		wearer.job = 100;
		wearer.stats = { 10, 4, 4, 4 };
		Worn sword = worn(11, 1302000, 0, sword_needs);
		Worn gloves = worn(8, 1082002, 5, {});
		require(counted(wearer, { sword, gloves }) == std::vector<bool>{ true, true }, "the gloves' STR holds the sword");
		require(counted(wearer, { sword }) == std::vector<bool>{ false }, "without them it fails");
		wearer.percent = 50;
		require(counted(wearer, { sword }) == std::vector<bool>{ true }, "Maple Warrior's STR counts");
		wearer.percent = 0;

		wearer.stats[0] = 5;
		Worn shield = worn(10, 1092000, 10, shield_needs);
		require(counted(wearer, { shield }) == std::vector<bool>{ false }, "an item never holds itself up");
		require(counted(wearer, { shield, gloves }) == std::vector<bool>{ true, true }, "others' STR does");

		// The shield (5 + 5) lets the sword pass, but fails at 5 itself;
		// the sword then fails too.
		Worn weak = worn(10, 1092000, 5, shield_needs);
		Worn bare = worn(8, 1082002, 0, {});
		require(counted(wearer, { sword, weak, bare }) == std::vector<bool>{ false, false, true },
				"what drops takes its STR from the rest");

		wearer.stats[0] = 20;
		Worn cash = worn(111, 1702000, 0, {});
		require(counted(wearer, { cash }) == std::vector<bool>{ false }, "a cash weapon needs a weapon under it");
		require(counted(wearer, { cash, sword }) == std::vector<bool>{ true, true }, "and counts over one");
		wearer.stats[0] = 10;
		require(counted(wearer, { cash, sword }) == std::vector<bool>{ false, false }, "but not over one that fails");

		Worn pet = worn(14, 1802000, 0, warriors);
		require(counted(wearer, { pet }) == std::vector<bool>{ true }, "pet equipment isn't checked");
	}
	catch (const std::exception& e)
	{
		std::cerr << "FAIL: " << e.what() << "\n";
		return 1;
	}

	std::cout << "equip_count: all passed\n";
	return 0;
}
