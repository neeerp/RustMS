#include "../Character/Inventory/CashUpgrades.h"
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
		using vicious_hammer::Refusal;

		require(vicious_hammer::check(true, 1082002, 5, false, 0) == Refusal::NONE, "gloves take a hammer");
		require(vicious_hammer::check(true, 1082002, 5, false, 1) == Refusal::NONE, "and a second");
		require(vicious_hammer::check(true, 1082002, 5, false, 2) == Refusal::USED_UP, "but not a third");
		require(vicious_hammer::check(false, 2000000, 0, false, 0) == Refusal::NOT_UPGRADABLE, "a potion is refused");
		require(vicious_hammer::check(false, 1082002, 5, false, 0) == Refusal::NOT_UPGRADABLE,
				"only from the Equip tab");
		require(vicious_hammer::check(true, 1122000, 5, false, 0) == Refusal::HORNTAIL, "Horntail's Necklace says so");
		require(vicious_hammer::check(true, 1122001, 5, false, 0) == Refusal::NONE, "only 1122000 is Horntail's");
		require(vicious_hammer::check(true, 1142000, 0, false, 0) == Refusal::NOT_UPGRADABLE,
				"a medal without slots is refused");
		require(vicious_hammer::check(true, 1802000, 5, false, 0) == Refusal::NOT_UPGRADABLE, "a pet's equip is refused");
		require(vicious_hammer::check(true, 1082002, 5, true, 0) == Refusal::NOT_UPGRADABLE,
				"a Cash Shop equip is refused");
		require(vicious_hammer::check(true, 1122000, 0, false, 2) == Refusal::HORNTAIL, "Horntail comes before the rest");

		require(vicious_hammer::takes(1082002, 5), "the tooltip counts hammers on gloves");
		require(!vicious_hammer::takes(1122000, 5), "but not on Horntail's Necklace");
		require(!vicious_hammer::takes(1142000, 0), "nor on a medal");
		require(vicious_hammer::tooltip_line(1) == "1", "one hammer");
		require(vicious_hammer::tooltip_line(2) == "2(MAX)", "two is the most");
		require(vicious_hammer::done_text(1) == "Increased available upgrade by 1. \\n1 upgrades are left.",
				"one left after the first");
		require(vicious_hammer::error_text(3) == "Unknown Error (3).", "the error names its code");

		require(vega_spell::is_vega(5610000) && vega_spell::is_vega(5610001), "both spells");
		require(!vega_spell::is_vega(5570000), "the hammer is no spell");
		require(vega_spell::takes_scroll(2040802), "a gloves scroll");
		require(!vega_spell::takes_scroll(2049100), "not Chaos");
		require(!vega_spell::takes_scroll(2041211) && !vega_spell::takes_scroll(2041212), "nor the two cape scrolls");
		require(!vega_spell::takes_scroll(2340000), "nor the White Scroll");
		require(vega_spell::raised(5610000, 10) == 30, "10 goes to 30");
		require(vega_spell::raised(5610001, 60) == 90, "60 goes to 90");
		require(vega_spell::raised(5610000, 60) == 0, "the 10% spell is not for a 60% scroll");
		require(vega_spell::raised(5610001, 10) == 0, "nor the 60% one for a 10% scroll");
	}
	catch (const std::exception& e)
	{
		std::cerr << "FAILED: " << e.what() << std::endl;
		return 1;
	}

	std::cout << "cash upgrade tests passed" << std::endl;
	return 0;
}
