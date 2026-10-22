#include "../Data/CashCoupon.h"
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
		using namespace ms::cash_coupon;

		require(trim("  ABC 12  ") == "ABC 12", "spaces around the code go");
		require(trim("   ").empty() && trim("").empty(), "only spaces is no code");

		require(without_hyphen("AB12-") == "AB12", "a typed hyphen goes");
		require(without_hyphen("AB-12") == "AB-12", "only one at the end");
		require(without_hyphen("-").empty(), "a lone hyphen goes");

		require(std::string(failure(0xB3)) == "This coupon was already used.", "a used coupon");
		require(std::string(failure(0xB2)) == "This coupon has expired.", "an expired coupon");
		require(failure(0xB0) != nullptr && failure(0xD2) != nullptr, "coupon codes have texts");
		require(failure(0xB1) == nullptr, "the third wrong code gets the default");
		require(failure(0xBB) == nullptr && failure(0xA5) == nullptr, "the shop's other codes aren't coupons'");

		std::string items = notice({ { 1, "Safety Charm" }, { 5, "Red Potion" } }, 0, 0);
		require(items == "You have received\\n1 Safety Charm\\n5 Red Potion\\nusing the coupon.", "items, one to a line");

		require(notice({ { 1, "A" }, { 5, "B" } }, 200, 1000) == "You have received\\n1 A\\n5 Band \\n200 MaplePointsand \\n1000 mesos\\nusing the coupon.", "everything, as v83 glues it");
		require(notice({}, 200, 0) == "You have received200 MaplePoints\\nusing the coupon.", "points alone");
		require(notice({}, 0, 1000) == "You have received1000 mesos\\nusing the coupon.", "mesos alone");
		require(notice({}, 200, 1000) == "You have received200 MaplePointsand \\n1000 mesos\\nusing the coupon.", "points and mesos");
		require(notice({}, 0, 0) == "You have received\\nusing the coupon.", "nothing at all");
	}
	catch (const std::exception& e)
	{
		std::cerr << "FAILED: " << e.what() << std::endl;
		return 1;
	}

	std::cout << "cash_coupon: all tests passed" << std::endl;
	return 0;
}
