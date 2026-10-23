#include "../Net/Handlers/WeddingGifts.h"
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
		return put<int8_t>(2).put(itemid).put<int8_t>(0).put<int64_t>(0).put(count).string("").put<int16_t>(0);
	}

	Writer& equip(int32_t itemid)
	{
		put<int8_t>(1).put(itemid).put<int8_t>(0).put<int64_t>(0).put<int8_t>(7).put<int8_t>(0);

		for (int i = 0; i < 15; i++)
			put<int16_t>(0);

		string("").put<int16_t>(0);
		put<int8_t>(0).put<int8_t>(1).put<int16_t>(0).put<int16_t>(0).put<int32_t>(0).put<int64_t>(0);
		return put<int64_t>(0).put<int32_t>(-1);
	}
};

int main()
{
	try
	{
		using namespace ms;

		// The Registry opens on the wishlist.
		Writer list;
		list.put<int8_t>(2).string("Potions").string("Cake");
		InPacket list_recv(list.bytes.data(), list.bytes.size());
		std::vector<std::string> wishes = wedding_gifts::read_wishlist(list_recv);

		require(!list_recv.available(), "the whole wishlist is read");
		require(wishes == std::vector<std::string>({ "Potions", "Cake" }), "the wishes are read in order");

		// The gift window opens on every type.
		Writer all;
		all.put<int64_t>(0x7c);
		all.put<int8_t>(1).equip(1302000);
		all.put<int8_t>(2).stack(2000000, 4).stack(2000001, 1);
		all.put<int8_t>(0).put<int8_t>(0).put<int8_t>(0);
		InPacket all_recv(all.bytes.data(), all.bytes.size());
		wedding_gifts::Gifts gifts;
		wedding_gifts::read_gifts(all_recv, gifts);

		require(!all_recv.available(), "every type is read");
		require(gifts[1].size() == 1 && gifts[1][0].itemid == 1302000, "an equip gift is read");
		require(gifts[2].size() == 2 && gifts[2][0].count == 4 && gifts[2][1].itemid == 2000001,
			"use gifts are read in order");
		require(gifts[3].empty() && gifts[4].empty() && gifts[5].empty(), "empty types are empty");

		// A take answers with the type that changed; the others stay.
		Writer use;
		use.put<int64_t>(0x08).put<int8_t>(1).stack(2000001, 1);
		InPacket use_recv(use.bytes.data(), use.bytes.size());
		wedding_gifts::read_gifts(use_recv, gifts);

		require(!use_recv.available(), "one type is read");
		require(gifts[2].size() == 1 && gifts[2][0].itemid == 2000001, "the type is replaced");
		require(gifts[1].size() == 1, "the other types keep their gifts");

		std::cout << "wedding_gifts: all tests passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << "FAILED: " << error.what() << '\n';
		return 1;
	}
}
