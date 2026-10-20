#include "../Character/Inventory/ItemProperties.h"
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
		using namespace ms::item_properties;

		Info info;
		info.trade_block = true;
		require(equip(info, 0, false).first == "Untradeable" && bundle(info, 0).first == "Untradeable", "untradeable");

		info.only = true;
		require(bundle(info, 0).first == "One-of-a-kind Item, Untradeable", "joined");

		info.trade_block = false;
		info.quest = true;
		info.pquest = true;
		require(bundle(info, 0).first == "One-of-a-kind Item, Quest Item, Party Quest Item", "in order");

		Info karma;
		karma.trade_block = true;
		require(equip(karma, EQUIP_KARMA, false).first == "Can trade 1 time (Trade disabled after transaction)",
			"karma alone takes the first line");
		require(bundle(karma, BUNDLE_KARMA).first == "Can trade 1 time (Trade disabled after transaction)",
			"bundle karma bit");
		require(bundle(karma, EQUIP_KARMA).first == "Untradeable", "equip karma bit means nothing to a bundle");

		karma.only = true;
		Lines both = equip(karma, EQUIP_KARMA, false);
		require(both.first == "One-of-a-kind Item" &&
				both.second == "Can trade 1 time (Trade disabled after transaction)", "karma takes the second line");

		Info flagged;
		flagged.equip_trade_block = true;
		require(equip(flagged, 0, false).first == "Trade disabled when equipped", "trade disabled when equipped");
		require(equip(flagged, EQUIP_UNTRADEABLE, false).first == "Untradeable", "the flag hides it");

		flagged.trade_block = true;
		require(equip(flagged, 0, false).first == "Untradeable, Trade disabled when equipped", "both from WZ");

		Info unique;
		unique.only_equip = true;
		unique.account_sharable = true;
		require(equip(unique, 0, false).first == "Unique Equipped Item, Movement is possible only within the account",
			"account joins the first line");

		unique.only = true;
		unique.not_extend = true;
		Lines spilled = equip(unique, 0, true);
		require(spilled.first == "One-of-a-kind Item, Movement is possible only within the account" &&
				spilled.second == "Unique Equipped Item, Time limit extension is not possible.",
			"the rest join the second line");
		require(equip(unique, 0, false).second == "Unique Equipped Item", "no expiry, no extension line");

		require(item_properties::karma(1032031, EQUIP_KARMA) && !item_properties::karma(1032031, BUNDLE_KARMA),
			"equip karma bit");
		require(item_properties::karma(2210005, BUNDLE_KARMA) && !item_properties::karma(2210005, EQUIP_KARMA),
			"bundle karma bit");

		require(with_karma_hint("").find("#c") == 0, "hint alone");
		require(with_karma_hint("A ring.") == std::string("A ring.\\n") + KARMA_HINT, "hint on its own line");

		std::cout << "item properties passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
