#include "../Character/Inventory/MakerRules.h"
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
		using namespace ms;
		using namespace ms::maker;

		require(skill_id(0) == 1007, "a Beginner's Maker");
		require(is_maker(1007) && is_maker(10001007) && is_maker(20001007) && is_maker(20011007), "the four Makers");
		require(!is_maker(1008) && !is_maker(11007), "other skills are not");
		require(skill_id(412) == 1007, "an explorer's");
		require(skill_id(1100) == 10001007, "a Cygnus Knight's");
		require(skill_id(2112) == 20001007, "an Aran's");
		require(skill_id(2001) == 20011007, "an Evan's");
		require(skill_id(2218) == 20011007, "a later Evan's");

		require(groups(0) == std::vector<int32_t>{ 0 }, "a Beginner sees group 0");
		require(groups(112) == std::vector<int32_t>({ 0, 1 }), "a warrior sees 1");
		require(groups(232) == std::vector<int32_t>({ 0, 2 }), "a magician 2");
		require(groups(312) == std::vector<int32_t>({ 0, 4 }), "a bowman 4");
		require(groups(412) == std::vector<int32_t>({ 0, 8 }), "a thief 8");
		require(groups(522) == std::vector<int32_t>({ 0, 16 }), "a pirate 16");
		require(groups(1311) == std::vector<int32_t>({ 0, 4 }), "a Wind Archer as a bowman");
		require(groups(910) == std::vector<int32_t>({ 0, 1, 2, 4, 8, 16 }), "a GM sees them all");

		require(category(1302010) == 130, "a sword under its kind");
		require(category(4250000) == GEMS, "gems apart");
		require(category(2040727) == USE, "a scroll under Use");
		require(category(3010000) == SETUP, "a chair under Setup");
		require(category(4260000) == ETC, "a crystal under Etc");
		require(category(4032312) == ETC, "an order form under Etc");

		require(quest_met(1, 1) && !quest_met(1, 2), "1 wants it started");
		require(quest_met(2, 2) && !quest_met(2, 1), "2 wants it done");
		require(quest_met(3, 1) && quest_met(3, 2) && !quest_met(3, 0), "3 either");

		require(fee(100000, false, {}) == 110000, "a tenth more");
		require(fee(55000, false, {}) == 60000, "floored to a thousand");
		require(fee(60000, true, { 4250100, 4250002 }) == 60000 + 18 * 6000, "a stimulator and two gems");
		require(fee(0, false, {}) == 0, "free");

		require(count_text(3, 1) == "3/1", "owned over needed");
		require(confirm_text(1, "Basic Diamond") == "Basic Diamond: Would you like to make this?", "a recipe");
		require(confirm_text(3, "") == "Would you like to make a Monster Crystal?", "a crystal");
		require(result_text(0, 1, "Basic Diamond") == "1 of Basic Diamond  have been produced.", "made");
		require(result_text(0, 1, "").empty(), "nothing named, nothing said");
		require(result_text(2, 0, "") == "Production failed due to an unknown error.", "failed");
		require(result_text(7, 0, "") == "Etc You don't have enough room in your Inventory.", "the tab first");
		require(result_text(3, 0, "").empty(), "3 says nothing");
		require(gained_line("Etc", "Basic Diamond", 1) == "You have gained items in the Etc tab (Basic Diamond 1)",
				"always plural");
		require(lost_line("Diamond Ore", 1) == "You have lost items (Diamond Ore 1)", "a lost ingredient");
		require(meso_line(110000) == "You have lost mesos. (-110000)", "the fee");

		auto recipe = [](int32_t id)
		{
			Recipe r;
			r.item_id = id;
			r.req_skill_level = 1;
			return r;
		};

		Maker maker;
		maker.level = 50;
		maker.maker_level = 1;
		maker.holds = [](int32_t id) { return id == 4031966; };
		maker.wears = [](int32_t id) { return id == 1122013; };
		maker.quest_state = [](int32_t quest) { return quest == 21303 ? 1 : 0; };

		Recipe plain = recipe(4250000);
		require(listed(plain, maker), "a plain recipe");
		plain.req_level = 51;
		require(!listed(plain, maker), "too low a level");
		plain = recipe(4250000);
		plain.req_skill_level = 2;
		require(!listed(plain, maker), "too low a Maker");
		plain = recipe(4001174);
		plain.req_item = 4031966;
		require(listed(plain, maker), "the item held");
		plain.req_item = 4031967;
		require(!listed(plain, maker), "the item not held");
		plain = recipe(2040727);
		plain.req_equip = 1122013;
		require(listed(plain, maker), "the equipment worn");
		plain.req_equip = 1122014;
		require(!listed(plain, maker), "the equipment not worn");
		plain = recipe(4032312);
		plain.req_quests = { { 21303, 2 } };
		require(!listed(plain, maker), "the quest not done");
		plain.req_quests = { { 21303, 3 } };
		require(listed(plain, maker), "started is enough for 3");

		Recipe sword = recipe(1302010), cap = recipe(1002000), sword2 = recipe(1302011), gem = recipe(4250000),
			   scroll = recipe(2040727), crystal = recipe(4260000);
		std::vector<const Recipe*> all = { &sword, &gem, &cap, &scroll, &sword2, &crystal };
		require(categories(all) == std::vector<int32_t>({ 0, DISASSEMBLE, CRYSTAL, GEMS, 130, 100, USE, ETC, RANDOM }),
				"in their order, Setup left out");
		require(categories({ &sword }) == std::vector<int32_t>({ 0, DISASSEMBLE, CRYSTAL, 130, RANDOM }),
				"no gems, no gem entry");
		require(under(130, all) == std::vector<const Recipe*>({ &sword, &sword2 }), "both swords");
		require(under(ETC, all) == std::vector<const Recipe*>({ &crystal }), "the crystal under Etc");

		std::vector<CrystalLevels> crystals = { { 4260000, 31, 50 }, { 4260001, 51, 60 } };
		require(crystal_for(35, crystals) == 4260000, "level 35");
		require(crystal_for(55, crystals) == 4260001, "level 55");
		require(crystal_for(30, crystals) == 0, "too low");
		require(equip_category(1002000) == "HAT" && equip_category(1302000) == " ONE-HANDED SWORD", "equipment names");
		require(equip_category(1322000) == " ONE_HANDED MACE" && equip_category(1040000) == "TOP", "as the pool has them");
		require(equip_category(1492000) == "GUN" && equip_category(1342000).empty() && equip_category(2000000).empty(),
			"no name");
		require(crystal_for(0, crystals) == 0, "no level");

		require(is_weapon(1302010) && is_weapon(1492000) && !is_weapon(1002000) && !is_weapon(1342000), "weapons");
		require(gem_kind(4250001) == 0 && gem_kind(4250502) == 5 && gem_kind(4251402) == 14, "gem kinds");
		require(gem_refusal({}, 1, 4250001, true).empty(), "an attack gem on a sword");
		require(gem_refusal({}, 1, 4250001, false) == "You can only use this Strengthening Gem on weapon items.",
				"not on a hat");
		require(gem_refusal({ 4250502 }, 1, 4250001, true) == "You cannot apply the Strengthening Gem anymore.",
				"the slots are full");
		require(gem_refusal({ 4250502 }, 1, 4011000, true) == "You cannot apply the Strengthening Gem anymore.",
				"full, and not a gem");
		require(gem_refusal({}, 2, 4011000, true) == "You can only apply the Strengthening Gem.", "not a gem");
		require(gem_refusal({ 4250500 }, 2, 4250502, false) ==
					"You cannot apply more than 1 of the same Strengthening Gems.",
				"one of a kind");
		require(stimulator_refusal(false, 4130002, 4130002).empty(), "its stimulator");
		require(stimulator_refusal(true, 4130002, 4130002) == "You can only apply 1 Stimulator.", "one only");
		require(stimulator_refusal(false, 4011000, 4130002) == "You can only apply the Stimulator.", "not one");
		require(stimulator_refusal(false, 4130001, 4130002) == "You cannot apply this Stimulator.", "another");
		Recipe made = recipe(1302010);
		made.meso = 60000;
		made.ingredients = { { 4011000, 1 }, { 4260000, 20 }, { 4260001, 1 } };
		require(disassemblable(made), "a sword comes apart");
		made.item_id = 4250000;
		require(!disassemblable(made), "a gem doesn't");
		std::vector<int32_t> own = { 0, 0, 0, 0, 0, 0, 67, 0, 0, 0, 0, 0, 0, 0 };
		std::vector<int32_t> stats = own;
		require(stat_grade(stats, own) == 0, "its own stats");
		stats[6] = 66;
		require(stat_grade(stats, own) == -1, "below them");
		stats[6] = 73;
		stats[4] = 19;
		require(stat_grade(stats, own) == 1, "a tenth of HP counts");
		stats[0] = 17;
		require(stat_grade(stats, own) == 2, "23 above");
		stats[1] = 17;
		require(stat_grade(stats, own) == 3, "40 above");
		require(disassembly_fee(60000, -1) == 6000 && disassembly_fee(60000, 0) == 9000 &&
					disassembly_fee(60000, 3) == 18000 && disassembly_fee(0, 0) == 0,
				"the disassembly fee by grade");
		require(disassembly_fee(66500, 1) == 13000, "rounded down to 1000");
		require(no_room_text(4) == "Etc You don't have enough room in your Inventory." &&
					result_text(4, 0, "") == "Eqp You don't have enough room in your Inventory.",
				"no room");
		require(disassembled_text("Sword") == "You have successfully disassembled the Sword.", "disassembled");
		require(lost_item_line("Sword") == "You have lost an item (Sword)", "lost an item");
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}

	return 0;
}
