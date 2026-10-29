#include "../Data/CashGift.h"
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
		using namespace ms::cash_gift;

		Equip sword;
		sword.job = 1;
		sword.level = 10;
		Equip famous;
		famous.fame = 5;
		Lookup equip;
		equip.equip = [&](int32_t id) -> std::optional<Equip> {
			switch (id)
			{
			case 1302000: return sword;
			case 1002000: return famous;
			case 1702000: case 1902000: case 1050000: return Equip{};
			default: return std::nullopt;
			}
		};
		// The cash sword has art for one-handed swords and axes.
		equip.covers = [](int32_t, int32_t weapon) { return weapon / 10000 == 130 || weapon / 10000 == 131; };

		Receiver receiver;
		receiver.name = "Alex";
		receiver.job = 100;
		receiver.level = 10;

		require(!use_check(1302000, receiver, equip), "a level 10 warrior uses the sword");
		receiver.level = 9;
		auto low = use_check(1302000, receiver, equip);
		require(low && low->ask && low->text.find("Alex can't") != std::string::npos, "short of its level asks, naming the receiver");
		receiver.level = 10;
		receiver.job = 2110;
		require(!use_check(1302000, receiver, equip), "an Aran is a warrior's class");
		receiver.job = 0;
		require(use_check(1302000, receiver, equip).has_value(), "a beginner isn't");
		receiver.job = 200;
		require(use_check(1302000, receiver, equip).has_value(), "nor a magician");
		require(use_check(1002000, receiver, equip).has_value() && (receiver.fame = 5, !use_check(1002000, receiver, equip)), "fame counts when it's asked for");
		require(!use_check(1050000, receiver, equip), "an equip without requirements");

		auto missing = use_check(1049999, receiver, equip);
		require(missing && !missing->ask && missing->text == UNKNOWN_ERROR, "an unknown equip ends the gift");

		require(!use_check(1702000, receiver, equip), "no weapon worn, no question");
		receiver.weapon = 1302000;
		require(!use_check(1702000, receiver, equip), "a one-handed sword fits");
		receiver.weapon = 1452000;
		auto bow = use_check(1702000, receiver, equip);
		require(bow && bow->text.find("weapon that Alex") != std::string::npos, "a bow doesn't");

		require(use_check(1902000, receiver, equip).has_value(), "a mount needs Monster Riding");
		receiver.riding = true;
		require(!use_check(1902000, receiver, equip), "and has it");

		require(use_check(5000000 + 190000, receiver, equip).has_value(), "pet food needs a pet");
		receiver.pet = true;
		require(!use_check(5240000, receiver, equip), "and has one");
		auto scissors = use_check(5520000, receiver, equip);
		require(scissors && scissors->text.find("Alex does not have") == 0, "the Scissors need an item to work on");
		receiver.karma_target = true;
		require(!use_check(5520000, receiver, equip), "and have one");
		require(!use_check(2000000, receiver, equip) && !use_check(5150000, receiver, equip), "nothing else is checked");

		receiver.level = 1;
		auto item = questions(false, { { 1302000, 90 } }, receiver, equip);
		require(item.size() == 2 && item[0].text == EXPIRES && item[1].ask, "an item asks about its period, then its use");
		auto package = questions(true, { { 1050000, 0 }, { 1302000, 0 }, { 1702000, 90 } }, receiver, equip);
		require(package.size() == 1 && package[0].text.find("job limitations") != std::string::npos, "a package asks for its first member that needs it");
		auto expiring = questions(true, { { 1702000, 90 }, { 1302000, 0 } }, receiver, equip);
		require(expiring.size() == 1 && expiring[0].text == EXPIRES, "an expiring member asks only that");
		receiver.level = 10;
		receiver.job = 110;
		require(questions(true, { { 1050000, 0 }, { 1302000, 0 } }, receiver, equip).empty(), "a package anyone can use asks nothing");
	}
	catch (const std::exception& e)
	{
		std::cerr << "FAIL: " << e.what() << "\n";
		return 1;
	}

	std::cout << "cash_gift: all passed\n";
	return 0;
}
