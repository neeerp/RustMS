#include "../Net/Handlers/PersonalShop.h"
#include <cstring>
#include <iostream>
#include <stdexcept>
#include <string>
#include <vector>

void require(bool condition, const char* message)
{
	if (!condition) throw std::runtime_error(message);
}

struct Writer
{
	std::vector<int8_t> bytes;

	template <typename T>
	Writer& put(T value)
	{
		int8_t raw[sizeof(T)];
		std::memcpy(raw, &value, sizeof(T));
		bytes.insert(bytes.end(), raw, raw + sizeof(T));
		return *this;
	}

	Writer& string(const std::string& text)
	{
		put<int16_t>(static_cast<int16_t>(text.size()));
		bytes.insert(bytes.end(), text.begin(), text.end());
		return *this;
	}

	Writer& stack(int32_t itemid, int16_t count)
	{
		put<int8_t>(2).put(itemid).put<int8_t>(0).put<int64_t>(0);
		return put(count).string("").put<int16_t>(0);
	}
};

// Stands in for the AvatarLook parser: a look here is just its hair.
ms::LookEntry read_hair(ms::InPacket& recv)
{
	ms::LookEntry look = {};
	look.hairid = recv.read_int();

	return look;
}

ms::mini_room::Listing listing(int32_t itemid, int16_t bundles, int16_t per_bundle, int32_t price)
{
	ms::mini_room::Listing row;
	row.bundles = bundles;
	row.per_bundle = per_bundle;
	row.price = price;
	row.item.itemid = itemid;
	row.item.count = per_bundle;

	return row;
}

int main()
{
	try
	{
		using namespace ms;
		using namespace ms::personal_shop;

		// A visitor comes into Alice's shop, which sells potions in tens.
		Writer shop;
		shop.put<int8_t>(4).put<uint8_t>(4).put<uint8_t>(1);
		shop.put<uint8_t>(0).put<int32_t>(30000).string("Alice");
		shop.put<uint8_t>(1).put<int32_t>(30010).string("Bob");
		shop.put<uint8_t>(0xFF);
		shop.string("Cheap pots").put<uint8_t>(16);
		shop.put<uint8_t>(2);
		shop.put<int16_t>(3).put<int16_t>(10).put<int32_t>(1500).stack(2000000, 10);
		shop.put<int16_t>(0).put<int16_t>(1).put<int32_t>(25000000).stack(4000000, 1);
		InPacket shop_recv(shop.bytes.data(), shop.bytes.size());
		mini_room::Enter enter = mini_room::read_enter(shop_recv, read_hair);

		require(!shop_recv.available(), "a shop's enter is read up to its end");
		require(enter.type == mini_room::PERSONAL_SHOP && enter.max_users == 4 && enter.own_slot == 1, "the shop and our seat are read");
		require(enter.members.size() == 2 && enter.members[0].name == "Alice", "the owner sits first");
		require(enter.title == "Cheap pots" && enter.max_items == 16, "the title and room for rows are read");
		require(enter.listings.size() == 2, "the rows are read");
		require(enter.listings[0].bundles == 3 && enter.listings[0].per_bundle == 10, "a row's bundles are read");
		require(enter.listings[0].price == 1500 && enter.listings[0].item.itemid == 2000000, "a row's price and item are read");
		require(enter.listings[1].bundles == 0, "a sold out row stays");

		// The rows as the shop shows them.
		require(price_text(enter.listings[0]) == "10 for 1,500 mesos.", "stacks sell so many for a price");
		require(price_text(listing(1302000, 1, 1, 70000)) == "70,000 mesos.", "equips sell for a price");
		require(price_text(listing(2070000, 1, 1, 9000)) == "9,000 mesos.", "stars sell for a price");
		require(shows_count(2000000) && shows_count(4000000) && shows_count(3010000), "use, setup and etc show a count");
		require(!shows_count(1302000) && !shows_count(2070000) && !shows_count(2330000), "equips, stars and bullets don't");
		require(price_tier(99999) == 0 && price_tier(100000) == 1 && price_tier(999999) == 1, "prices under a million");
		require(price_tier(1000000) == 2 && price_tier(10000000) == 3 && price_tier(100000000) == 4, "prices by digits");
		require(price_tier(1000000000) == 5 && price_tier(2147483647) == 5, "a billion and more");

		// The fee comes off what the owner receives.
		require(fee(99999) == 0 && fee(100000) == 400, "small sales go free");
		require(fee(1000000) == 9000 && fee(5000000) == 75000, "the fee grows with the sale");
		require(fee(10000000) == 200000 && fee(25000000) == 625000 && fee(100000000) == 3000000, "the top rates");

		Writer sold;
		sold.put<uint8_t>(0).put<int16_t>(2).string("Bob");
		InPacket sold_recv(sold.bytes.data(), sold.bytes.size());
		SoldNotice notice = read_sold(sold_recv);

		require(!sold_recv.available() && notice.index == 0 && notice.bundles == 2 && notice.buyer == "Bob", "a sale is read");

		Sales sales;
		sales.add(listing(2000000, 3, 10, 1500), 2, "Bob");
		sales.add(listing(1302000, 1, 1, 1000000), 1, "Carol");

		require(sales.sold.size() == 2 && sales.sold[0].units == 20 && sales.sold[0].buyer == "Bob", "a sale counts its items");
		require(sales.total == 1003000 && sales.received == 1003000 - 9000, "the totals keep the fee per sale");

		// Selling: the boxes are checked bundle size first, then the count,
		// then the price.
		SellCheck stack = check_sale({ "100", "1500", "1", false }, 2000000, 100);

		require(stack.box == Box::NONE && stack.sale.bundles == 100 && stack.sale.per_bundle == 1, "loose items sell one a bundle");
		require(stack.sale.price == 1500, "the price is read");

		SellCheck tens = check_sale({ "100", "1500", "10", true }, 2000000, 100);

		require(tens.box == Box::NONE && tens.sale.bundles == 10 && tens.sale.per_bundle == 10, "bundles divide the count");

		SellCheck uneven = check_sale({ "95", "1500", "10", true }, 2000000, 100);

		require(uneven.box == Box::COUNT && uneven.problem == BUNDLE_ONLY, "the count must fill whole bundles");
		require(check_sale({ "10", "1500", "200", true }, 2000000, 100).box == Box::PER_BUNDLE, "a bundle can't be over the stack");
		require(check_sale({ "101", "1500", "1", false }, 2000000, 100).box == Box::COUNT, "the count can't be over the stack");
		require(check_sale({ "x", "1500", "1", false }, 2000000, 100).problem == "Only numbers are allowed.", "the count is a number");
		require(check_sale({ "10", "0", "1", false }, 2000000, 100).box == Box::PRICE, "the price is at least 1");

		SellCheck sword = check_sale({ "1", "70000", "1", false }, 1302000, 1);

		require(sword.box == Box::NONE && sword.sale.bundles == 1 && sword.sale.per_bundle == 1, "an equip sells whole");
		require(sells_whole(1302000) && sells_whole(2070000) && !sells_whole(2000000), "equips and stars sell whole");
		require(sells_whole(5072000) && sells_whole(2330000) && !sells_whole(4000000) && !sells_whole(3010000),
				"so do cash items and bullets, but not etc or setup stacks");
		require(sells_whole(2000000, ms::item_expiry::BEFORE - 1), "a stack with a time limit sells whole");
		require(ms::mini_room::time_limit_line("Hat", ms::item_expiry::BEFORE, ms::item_expiry::BEFORE) +
				ms::mini_room::CHECK_TIME_LIMIT == "Hat(Time Limit Expired)\\n\\nHave you checked the\\ntime limit on the item?",
				"a timed cash item is named before the question");
		require(too_pricey(2, 2000000000) && !too_pricey(1, 2147483647), "a sale can't total past what fits");

		// Buying takes whole bundles.
		Purchase three = purchase(listing(2000000, 3, 10, 1500), 30);

		require(three.problem.empty() && three.bundles == 3 && three.total == 4500, "units buy bundles");
		require(purchase(listing(2000000, 3, 10, 1500), 25).problem == BUNDLE_ONLY, "a part of a bundle can't be bought");
		require(!purchase(listing(2000000, 3, 1, 2000000000), 2).problem.empty(), "a purchase can't total past what fits");
		require(total_question(4500) == "The total price is 4500 mesos.\\nWill you make the purchase?", "the purchase asks");
		require(over_low_level_limit(15, 1000001) && !over_low_level_limit(15, 1000000), "low levels spend a million");
		require(!over_low_level_limit(16, 5000000), "higher levels spend freely");

		require(leave_text(3) == "The shop is closed." && leave_text(0).empty(), "the owner closing says so");
		require(buy_result_text(0).empty() && buy_result_text(2) == "You do not have enough mesos.", "a purchase's result");
		require(buy_result_text(0x7F) == "Due to an error, the trade did not happen.", "unknown results are an error");
		require(is_permit(5140000) && is_permit(5140006) && !is_permit(5150000), "permits are 514");

		// The balloon over the owner.
		Writer box;
		box.put<int8_t>(4).put<int32_t>(77).string("Cheap pots").put<int8_t>(0);
		box.put<uint8_t>(2).put<uint8_t>(1).put<uint8_t>(4).put<int8_t>(0);
		InPacket box_recv(box.bytes.data(), box.bytes.size());
		std::optional<MiniRoomBox> room = mini_room::read_box(box_recv);

		require(room && !box_recv.available(), "a balloon is read up to its end");
		require(room->type == 4 && room->serial == 77 && room->title == "Cheap pots", "the room is read");
		require(room->skin == 2 && room->users == 1 && room->max_users == 4, "the skin and seats are read");

		Writer none;
		none.put<int8_t>(0);
		InPacket none_recv(none.bytes.data(), none.bytes.size());

		require(!mini_room::read_box(none_recv) && !none_recv.available(), "no room is one byte");

		std::cout << "Personal shop passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
