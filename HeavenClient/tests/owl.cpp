#include "../Net/Handlers/OwlResults.h"
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

	ms::InPacket packet() const
	{
		return ms::InPacket(bytes.data(), bytes.size());
	}
};

Writer& row(Writer& w, const std::string& owner, int32_t room, uint8_t channel, int8_t type)
{
	w.string(owner).put<int32_t>(910000001).string("Shells");
	w.put<int32_t>(2).put<int32_t>(5).put<int32_t>(100000).put<int32_t>(room);
	return w.put<uint8_t>(channel).put<int8_t>(type);
}

void the_market_is_the_entrance_and_its_rooms()
{
	require(ms::owl::in_free_market(910000000), "the entrance");
	require(ms::owl::in_free_market(910000022), "the last room");
	require(!ms::owl::in_free_market(910000023), "past the rooms");
	require(!ms::owl::in_free_market(100000000), "Henesys");
}

void rows_read_in_order_with_an_equip_whole()
{
	Writer w;
	w.put<int32_t>(0).put<int32_t>(1302000).put<int32_t>(2);
	row(w, "Ann", 9, 1, 4);
	row(w, "Bob", 12, 0, 1);
	// An equip: type, id, not cash, expiration, slots, level, 15 stats, owner, flags, the rest.
	w.put<int8_t>(1).put<int32_t>(1302000).put<int8_t>(0).put<int64_t>(0);
	w.put<int8_t>(7).put<int8_t>(0);
	for (int i = 0; i < 15; i++)
		w.put<int16_t>(0);
	w.string("").put<int16_t>(0);
	for (int i = 0; i < 18 + 12; i++)
		w.put<int8_t>(0);
	w.put<int32_t>(0x7e7e7e7e);

	auto recv = w.packet();
	auto results = ms::owl::read_results(recv);
	require(results.itemid == 1302000, "the item");
	require(results.rows.size() == 2, "two rows");
	require(results.rows[0].owner == "Ann" && results.rows[0].title == "Shells", "names");
	require(results.rows[0].room == 9 && results.rows[0].channel == 1, "room and channel");
	require(results.rows[0].bundles == 2 && results.rows[0].per_bundle == 5, "bundles");
	require(results.rows[0].price == 100000 && !results.rows[0].equip, "price, no item block");
	require(results.rows[1].equip && results.rows[1].equip->itemid == 1302000, "the equip");
	require(recv.read_int() == 0x7e7e7e7e, "read to the end of the equip");
	require(!results.empty(), "not empty");
}

void no_rows_is_empty()
{
	Writer w;
	w.put<int32_t>(0).put<int32_t>(4000001).put<int32_t>(0);
	auto recv = w.packet();
	require(ms::owl::read_results(recv).empty(), "empty");
}

void the_popular_list_counts_its_items()
{
	Writer w;
	w.put<uint8_t>(2).put<int32_t>(4000000).put<int32_t>(2000000);
	auto recv = w.packet();
	require(ms::owl::read_popular(recv) == std::vector<int32_t>{ 4000000, 2000000 }, "the list");
}

void go_answers_say_why()
{
	require(ms::owl::go_text(0).empty(), "go says nothing");
	require(ms::owl::go_text(1) == "The room is already closed.", "closed");
	require(ms::owl::go_text(0x11) == "You may not enter this store.", "banned");
	require(ms::owl::go_text(5).empty(), "unread codes say nothing");
}

int main()
{
	the_market_is_the_entrance_and_its_rooms();
	rows_read_in_order_with_an_equip_whole();
	no_rows_is_empty();
	the_popular_list_counts_its_items();
	go_answers_say_why();
	std::cout << "owl tests passed\n";
}
