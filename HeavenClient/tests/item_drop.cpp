#include "../Character/Inventory/ItemDrop.h"
#include <iostream>
#include <string>
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
		using namespace ms::item_drop;

		require(refused(5160000, true), "a cash item stays in the inventory");
		require(refused(4220000, false), "a Flowerpot stays in the inventory");
		require(!refused(4000000, false), "a Snail Shell can be dropped");

		require(kept(1112803) && kept(2240000) && kept(4031481) && !kept(4031482), "wedding rings stay silently");

		Traits plain;
		require(steps(4000000, plain).empty(), "an ordinary item drops without a question");

		Traits quest;
		quest.quest = true;
		std::vector<Step> asked = steps(4031013, quest);
		require(asked.size() == 1 && asked[0].question, "Maria's Letter, a quest item, asks once");

		Traits egg = quest;
		egg.untradeable = true;
		asked = steps(4031047, egg);
		require(asked.size() == 2 && std::string(asked[0].text) == asked[1].text,
			"Nine Spirit's Egg, quest and untradeable, asks the same twice");

		egg.karma = true;
		asked = steps(4031047, egg);
		require(asked.size() == 1 && std::string(asked[0].text).find("Once dropped") == 0,
			"with Karma only the untradeable question asks, in its own words");

		Traits account;
		account.account = true;
		account.sealed = true;
		asked = steps(1122017, account);
		require(asked.size() == 2 && asked[0].question && !asked[1].question && asked[1].text == SEALED,
			"a sealed item asks first and is refused after");

		Traits card;
		card.untradeable = true;
		asked = steps(4300000, card);
		require(asked.size() == 1 && std::string(asked[0].text).find("card") != std::string::npos,
			"a New Year's Card asks only about the card");

		Traits bound = traits(item_properties::Info{}, true, false, item_properties::EQUIP_UNTRADEABLE,
			item_expiry::BEFORE);
		require(bound.untradeable && !bound.karma && !bound.expires, "an equip made untradeable");
		require(!traits(item_properties::Info{}, false, false, item_properties::EQUIP_UNTRADEABLE, 0).untradeable
			&& traits(item_properties::Info{}, false, false, item_properties::BUNDLE_KARMA, 0).karma,
			"a bundle has no untradeable flag but its own Karma flag");

		require(tradeable(plain) && !tradeable(quest) && !tradeable(bound), "untradeable items stay out of trades");
		Traits karma = bound;
		karma.karma = true;
		require(tradeable(karma), "Karma lets it trade once");
		require(!tradeable(account), "account items stay in the account");

		require(unsellable(plain) == nullptr && unsellable(karma) == nullptr, "a shop takes what may trade");
		require(unsellable(account) == SEALED, "a sealed item says so first");
		Traits pquest;
		pquest.pquest = true;
		pquest.karma = true;
		require(unsellable(pquest) == UNAVAILABLE, "party quest items stay even with Karma");

		require(storable(plain) && storable(account) && storable(karma), "storage takes account and Karma items");
		require(!storable(quest) && !storable(bound), "but not quest or untradeable ones without Karma");
		Traits cash;
		cash.cash = true;
		cash.karma = true;
		require(!storable(cash), "nor cash items");

		require(asks_count(4000000, 10), "a stack of Snail Shells asks how many");
		require(asks_count(2000000, 20), "a stack of potions asks how many");
		require(asks_count(3010000, 2), "a stack of chairs asks how many");
		require(!asks_count(4000000, 1), "a single item drops as one");
		require(!asks_count(2070000, 500), "throwing stars drop as a whole stack");
		require(!asks_count(2330000, 500), "bullets drop as a whole stack");
		require(!asks_count(1302000, -1), "equipment drops as one");
		require(!asks_count(5000000, 3), "cash items never ask");
		require(!asks_count(2022442, 5, true), "a stack with an expiry drops whole");

		std::cout << "Item drop passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
