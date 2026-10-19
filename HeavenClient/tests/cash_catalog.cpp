#include "../Data/CashCatalog.h"
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
		using namespace ms::cash_catalog;

		require(category(50200003) == 5 && subcategory(50200003) == 2, "an SN holds its tab and sub-tab");
		require(category(20100001) == 2 && subcategory(20100001) == 1, "Equip > Face");
		require(category(MAIN) == MAIN_CATEGORY && category(ETC) == 5, "tabs map to categories");

		require(label(2, 50200003) == HOT, "Class 2 is HOT");
		require(label(-1, 50200001) == NONE, "no Class, no label");
		require(label(-1, 10000001) == NEW, "Event > New is NEW");
		require(label(-1, 10100001) == NONE, "Event > Event has no default label");

		std::vector<Commodity> all = {
			{ 50200009, 5300000, 2000, 5, 2, NONE, 1 },
			{ 50200001, 5130000, 3200, 5, 2, NONE, 5 },
			{ 50200005, 5220000, 1000, 1, 2, HOT, 12 },
			{ 50200003, 5220000, 30000, 35, 2, HOT, 12 },
			{ 50200125, 5570000, 4900, 1, 2, HOT, 9 },
			{ 50100001, 5030000, 4000, 1, 2, NONE, 5 },
			{ 50200020, 1002000, 1000, 1, 0, NONE, 1 },
			{ 50200021, 1002001, 1000, 1, 1, NONE, 1 }
		};

		std::vector<Commodity> game = list(all, 5, 2, false);
		require(game.size() == 6, "a sub-tab holds its own items for the player's gender");
		require(game[0].sn == 50200003 && game[1].sn == 50200005 && game[2].sn == 50200125, "the highest priority comes first, then the lowest SN");
		require(game[3].sn == 50200001 && game[4].sn == 50200009 && game[5].sn == 50200020, "then lower priorities");
		require(list(all, 5, 2, true).back().sn == 50200021, "female players see female items");

		std::vector<Commodity> wished = find(all, { 50100001, 0, 99999999, 50200003 });
		require(wished.size() == 2 && wished[0].sn == 50100001 && wished[1].sn == 50200003, "the wish list keeps its order and skips empty slots");

		std::vector<int32_t> wishes = wish({ 7, 0, 0 }, 9);
		require(wishes == std::vector<int32_t>({ 7, 9 }), "a wish joins the end, dropping empty slots");
		require(wish(wishes, 7) == wishes, "a wish is listed once");
		require(wish({ 1, 2, 3, 4, 5, 6, 7, 8, 9, 10 }, 11).size() == WISH_LIST_SIZE, "the wish list holds ten");
		require(unwish({ 7, 9, 0 }, 7) == std::vector<int32_t>({ 9 }), "removing a wish keeps the rest");

		require(PAYMENTS[default_payment(1000, 5000, 5000, 5000)] == PREPAID, "NX Prepaid is checked first");
		require(PAYMENTS[default_payment(1000, 5000, 0, 0)] == CREDIT, "then whatever covers the price");
		require(PAYMENTS[default_payment(1000, 0, 1000, 0)] == MAPLE_POINTS, "Maple Points cover it exactly");
		require(PAYMENTS[default_payment(1000, 0, 0, 0)] == PREPAID, "nothing covers it");

		require(affordable(4000, 0, 4000, 0) && !affordable(4000, 3999, 3999, 3999), "any balance covering the price will do");

		require(advancement_slots(0, 1, 88, 4) == 0 && advancement_slots(0, 1, 92, 4) == 4,
			"a beginner keeps four Equip slots for an advancement");
		require(advancement_slots(0, 2, 84, 4) == 0 && advancement_slots(0, 4, 88, 4) == 8 && advancement_slots(0, 3, 92, 4) == 0,
			"and eight Use and Etc slots");
		require(advancement_slots(0, 2, 84, 8) == 8, "an expansion commodity adds eight");
		require(advancement_slots(100, 4, 92, 4) == 4 && advancement_slots(110, 2, 92, 4) == 4 && advancement_slots(100, 1, 92, 4) == 0,
			"warriors keep four Use and Etc slots");
		require(advancement_slots(111, 4, 92, 4) == 0, "third jobs aren't asked");
		require(advancement_slots(200, 4, 92, 4) == 4 && advancement_slots(300, 2, 92, 4) == 0, "magicians and bowmen keep Etc slots");
		require(advancement_slots(400, 2, 92, 4) == 4 && advancement_slots(400, 4, 92, 4) == 0, "thieves keep Use slots");
		require(advancement_slots(500, 2, 92, 4) == 4 && advancement_slots(1100, 4, 92, 4) == 4, "pirates and Cygnus warriors as warriors");
		require(advancement_slots(900, 2, 92, 4) == 0, "GMs aren't asked");

		require(page_count(0) == 1 && page_count(10) == 1 && page_count(11) == 2, "ten items to a page");

		std::cout << "Cash catalog passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
