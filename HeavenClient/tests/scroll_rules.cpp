#include "../Character/Inventory/ScrollRules.h"
#include <cstring>
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
		using namespace ms::scroll_rules;

		require(fits(2043001, 1302000), "a one-handed sword scroll fits a sword");

		require(is_legendary_spirit(1003) && is_legendary_spirit(10001003) && is_legendary_spirit(20001003),
				"the three Legendary Spirits");
		require(!is_legendary_spirit(1004), "Monster Rider isn't one");
		require(may_enchant(1302000, false), "the window takes a sword");
		require(!may_enchant(1702000, true), "but no cash weapon");
		require(may_enchant(1802000, true), "and pet equipment, cash or not");
		require(!fits(2043001, 1402000), "a one-handed sword scroll doesn't fit a two-handed one");
		require(fits(2040000, 1002067), "a helmet scroll fits a hat");
		require(!fits(2040000, 1302000), "a helmet scroll doesn't fit a sword");
		require(fits(2049100, 1302000), "Chaos fits a sword");
		require(fits(2049100, 1040002), "Chaos fits a top");
		require(!fits(2049100, 1802000), "Chaos doesn't fit pet equipment");
		require(fits(2049003, 1302000), "Clean Slate fits a sword");
		require(!fits(2049003, 1802000), "Clean Slate doesn't fit pet equipment");
		require(fits(2049200, 1112000), "an accessory scroll fits a ring");
		require(fits(2049200, 1122000), "an accessory scroll fits a pendant");
		require(fits(2049200, 1132000), "an accessory scroll fits a belt");
		require(!fits(2049200, 1142000), "an accessory scroll doesn't fit a medal");
		require(fits(2048000, 1802000), "a pet equipment scroll fits pet equipment");
		require(!fits(2049105, 1302000), "the 6th Anniversary scrolls fit nothing");
		require(fits(2049104, 1302000), "event Chaos fits a sword");
		require(fits(2049111, 1302000), "2049111 is Chaos again");
		require(!fits(2000000, 1302000), "a potion is not a scroll");
		require(!fits(2043001, 2000000), "a scroll only goes on equipment");

		require(is_scroll(2043000), "2043000 is a scroll");
		require(!is_scroll(WHITE_SCROLL), "the White Scroll is not dropped on equipment");

		require(takes_white_scroll(2043001), "a normal scroll offers a White Scroll");
		require(takes_white_scroll(2049100), "Chaos offers a White Scroll");
		require(!takes_white_scroll(2049000), "Clean Slate 1% offers none");
		require(!takes_white_scroll(2049003), "Clean Slate 20% offers none");
		require(takes_white_scroll(2049004), "2049004 is not Clean Slate");
		require(!takes_white_scroll(2040727), "the spikes scroll offers none");
		require(!takes_white_scroll(2041058), "the cold scroll offers none");

		require(std::strcmp(result_text(true, false, false),
			"The scroll lights up, and then its mysterious power has been transferred to the item.") == 0,
			"success");
		require(result_text(true, false, true)[0] == ' ', "success with a White Scroll keeps the leading space");
		require(std::strcmp(result_text(false, true, true),
			"The item is destroyed due to the overwhelming power of the scroll.") == 0,
			"a White Scroll doesn't change the destroyed line");
		require(std::strcmp(result_text(false, false, false),
			"The scroll lights up, but the item winds up as if nothing happened") == 0,
			"failure, without a period");
		require(std::strstr(result_text(false, false, true), "remained in tact.") != nullptr,
			"failure with a White Scroll");

		std::cout << "Scroll rules passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
