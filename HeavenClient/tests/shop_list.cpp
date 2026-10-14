#include "../Net/Handlers/ShopList.h"
#include <cstring>
#include <iostream>
#include <stdexcept>
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

	Writer& row(int32_t itemid, int32_t price)
	{
		return put(itemid).put(price).put<int32_t>(0).put<int32_t>(0).put<int32_t>(0);
	}
};

int main()
{
	try
	{
		using namespace ms;

		Writer w;
		w.put<int16_t>(4);
		w.row(2000000, 50).put<int16_t>(1).put<int16_t>(100);
		// 0.3 starts with the bytes 0x33 0x33, never a bundle size of 1.
		w.row(2070000, 500).put(0.3).put<int16_t>(530);
		// Recharge only.
		w.row(2070001, 0).put(0.4).put<int16_t>(500);
		w.row(2330000, 600).put(0.6).put<int16_t>(800);

		InPacket recv(w.bytes.data(), w.bytes.size());
		auto rows = shop_list::read(recv);

		require(!recv.available(), "the whole list is read");
		require(rows.size() == 4, "every row is read");
		require(rows[0].itemid == 2000000 && rows[0].price == 50 && rows[0].buyable == 100, "a potion is a stack");
		require(rows[1].itemid == 2070000 && rows[1].unitprice == 0.3 && rows[1].buyable == 530, "stars carry a recharge rate and a set size");
		require(rows[1].for_sale() && !rows[2].for_sale() && rows[2].unitprice == 0.4, "a recharge-only row is not for sale");
		require(rows[3].itemid == 2330000 && rows[3].index == 3, "buying refers to the server's row, past hidden ones");
		require(shop_list::is_rechargeable(2330000) && !shop_list::is_rechargeable(2060000), "bullets recharge, arrows don't");

		std::cout << "Shop list passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
