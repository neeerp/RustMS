#include "../Character/Inventory/KarmaScissors.h"
#include <iostream>
#include <stdexcept>
#include <string>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

int main()
{
	try
	{
		using namespace ms::karma_scissors;

		require(check({ 1302112, false, true, false }) == Drop::ACCEPTED, "an item with tradeAvailable is taken");
		require(check({ 2280004, false, true, false }) == Drop::ACCEPTED, "a skill book too");
		require(check({ 1302000, false, false, false }) == Drop::NOT_USABLE, "without it the scissors can't be used");
		require(check({ 1302112, false, true, true }) == Drop::UNLOCKED, "nor twice");
		require(check({ 1302112, true, false, true }) == Drop::SEALED, "a seal is named first");
		require(check({ 1302112, false, true, false, true }) == Drop::NOT_USABLE, "nor an equip a trade bound");
		require(std::string(refusal(Drop::SEALED)) == "You must break the seal first.", "the seal's notice");
		require(std::string(refusal(Drop::UNLOCKED)) == "This item has already been unlocked.", "the unlocked one");
	}
	catch (const std::exception& e)
	{
		std::cerr << "FAILED: " << e.what() << std::endl;
		return 1;
	}

	std::cout << "karma_scissors tests passed" << std::endl;
	return 0;
}
