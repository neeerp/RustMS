#include "../Character/Inventory/ItemGuard.h"
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
		using namespace ms::item_guard;

		require(is_guard(5060001) && is_guard(5061000) && is_guard(5061003), "the guards are guards");
		require(!is_guard(5060000) && !is_guard(5060002), "the tag and the incubator aren't");

		const int64_t day = 86400 * item_expiry::TICKS_PER_SECOND;
		const int64_t now = item_expiry::UNIX_EPOCH + 1800000000LL * item_expiry::TICKS_PER_SECOND;
		const int64_t never = item_expiry::BEFORE;

		Target sword { 1302000, true, false, never, false, false };
		require(check(sword) == Drop::ACCEPTED, "an equip is taken");
		require(check({ 2070000, false, false, never, false, false }) == Drop::ACCEPTED, "so are stars");
		require(check({ 2330000, false, false, never, false, false }) == Drop::ACCEPTED, "and bullets");
		require(check({ 2000000, false, false, never, false, false }) == Drop::REFUSED, "a potion isn't");
		require(check({ 4000000, false, false, never, false, false }) == Drop::REFUSED, "nor an etc item");
		require(check({ 1302000, true, false, now + day, false, false }) == Drop::REFUSED,
				"nor a time-limited equip");
		require(check({ 1302000, true, true, never, false, false }) == Drop::REFUSED, "nor one sealed for good");
		require(check({ 1302000, true, true, now + day, false, false }) == Drop::ACCEPTED,
				"a timed seal may be extended");
		require(check({ 1302000, true, false, never, true, false }) == Drop::REFUSED, "gone at logout is refused");
		require(check({ 1002000, true, false, never, false, true }) == Drop::REFUSED, "a cash equip too");
		require(check({ 1112300, true, false, never, false, false }) == Drop::REFUSED, "and 111230x");
		require(check({ 2000000, true, false, never, false, false }) == Drop::WARNED,
				"a potion class from the Equip tab is warned of");

		require(sealed_until(sword, 7, now) == now + 7 * day, "a week from now");
		require(sealed_until({ 1302000, true, true, now + 30 * day, false, false }, 7, now) == now + 37 * day,
				"a week past the seal's end");
		require(sealed_until({ 1302000, true, true, now - day, false, false }, 7, now) == now + 7 * day,
				"from now if the seal's end has passed");

		require(question("Sword") == "Sword\\nWould you like to seal this item?", "the lasting guard asks");
		require(timed_question("Sword", now + 7 * day, false) ==
					"Sword\\n will be sealed until 1-22-2027 08:00.\\nDo you want to proceed?",
				"the timed guard says until when");
		require(timed_question("Sword", now + 37 * day, true) ==
					"Sword\\n will extend the item seal until 2-21-2027 08:00.\\n Do you want to proceed?",
				"or how far it extends");
	}
	catch (const std::exception& e)
	{
		std::cerr << "FAILED: " << e.what() << std::endl;
		return 1;
	}

	std::cout << "item_guard tests passed" << std::endl;
	return 0;
}
