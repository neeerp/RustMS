#include "../Character/Inventory/EquipCash.h"
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
		using namespace ms::equip_cash;

		require(is_tag(5060000), "the Item Tag is a tag");
		require(!is_tag(5060001), "the Item Guard isn't");
		require(!is_tag(5061000), "nor is a timed guard");
		require(is_sandglass(5500000) && is_sandglass(5500006), "550xxxx are sandglasses");
		require(!is_sandglass(5510000), "551xxxx isn't");

		require(tag_question("Work Gloves") == "Woulid like to print your name on Work Gloves?", "the tag asks first");

		const int64_t day = 86400 * item_expiry::TICKS_PER_SECOND;
		const int64_t now = item_expiry::UNIX_EPOCH + 1800000000LL * item_expiry::TICKS_PER_SECOND;
		Sandglass week { 604800, 30 };
		Target target { now + 10 * day, false, false, false };

		require(extend(target, week, now) == Extend::ASK, "a week more on a 10-day equip is asked");
		require(extend({ now + 24 * day, false, false, false }, week, now) == Extend::TOO_FAR,
				"31 days from now is too far");
		require(extend({ now + 23 * day, false, false, false }, week, now) == Extend::ASK, "30 days is not");
		require(extend({ item_expiry::BEFORE, false, false, false }, week, now) == Extend::NONE,
				"a permanent equip is left alone");
		require(extend({ now + day, true, false, false }, week, now) == Extend::NONE, "so is a cash equip");
		require(extend({ now + day, false, true, false }, week, now) == Extend::NONE, "and a sealed one");
		require(extend({ now + day, false, false, true }, week, now) == Extend::NOT_EXTENDABLE,
				"notExtend says so");
		require(extend({ now + 100 * day, false, false, false }, { 604800, 0 }, now) == Extend::ASK,
				"no maxDays, no limit");

		require(added(86400) == "1 days", "days");
		require(added(7200) == "2 hrs.", "hours");
		require(added(600) == "10 min.", "minutes");
		require(extend_question("Work Gloves", 1728000) ==
				"When used on Work Gloves,  the effective date will be extended by 20 days. Would you like to use it?",
				"the sandglass asks first");
		require(too_far(30) == "You cannot extend the effective date beyond 30 days.", "the limit's notice");
	}
	catch (const std::exception& e)
	{
		std::cerr << "FAIL: " << e.what() << std::endl;
		return 1;
	}

	std::cout << "equip_cash: all passed" << std::endl;
	return 0;
}
