#include "../Net/Handlers/StorageContents.h"
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

	Writer& head(int8_t type, int32_t itemid)
	{
		return put(type).put(itemid).put<int8_t>(0).put<int64_t>(0);
	}

	Writer& stack(int32_t itemid, int16_t count, int16_t flags = 0)
	{
		head(2, itemid).put(count).string("").put<int16_t>(flags);

		if (itemid / 10000 == 207)
			put<int32_t>(2).put<int32_t>(0x34000054);

		return *this;
	}

	Writer& equip(int32_t itemid, int16_t flags = 0)
	{
		head(1, itemid).put<int8_t>(7).put<int8_t>(0);

		for (int i = 0; i < 15; i++)
			put<int16_t>(static_cast<int16_t>(i));

		string("owner").put<int16_t>(flags);
		put<int8_t>(0).put<int8_t>(1).put<int32_t>(0).put<int32_t>(0).put<int64_t>(0);
		return put<int64_t>(0).put<int32_t>(-1);
	}
};

int main()
{
	try
	{
		using namespace ms;

		// Open: everything, the way the server writes it.
		Writer w;
		w.put<int8_t>(6).put<int64_t>(0x7e).put<int32_t>(1234);
		w.put<int8_t>(1).equip(1302000, 0x10);
		w.put<int8_t>(2).stack(2000000, 3, 0x02).stack(2070000, 500);
		w.put<int8_t>(0);
		w.put<int8_t>(1).stack(4000000, 7);
		w.put<int8_t>(0);

		InPacket recv(w.bytes.data(), w.bytes.size());
		auto contents = storage_contents::read(recv);

		require(!recv.available(), "the whole storage is read");
		require(contents.slots == 6 && contents.meso == 1234, "slots and mesos are read");
		require(contents.bags[1]->size() == 1 && (*contents.bags[1])[0].itemid == 1302000, "an equip is read past its stats");
		require((*contents.bags[1])[0].count == 1, "an equip is one");
		require((*contents.bags[1])[0].flags == 0x10 && (*contents.bags[2])[0].flags == 0x02, "flags are read");
		require(contents.bags[2]->size() == 2 && (*contents.bags[2])[1].count == 500, "stars are read past their recharge");
		require(contents.bags[3]->empty() && contents.bags[5]->empty(), "empty bags are carried");
		require((*contents.bags[4])[0].itemid == 4000000 && (*contents.bags[4])[0].count == 7, "etc items are read");

		// A store or take-out carries one bag, a meso change only the mesos.
		Writer bag;
		bag.put<int8_t>(4).put<int64_t>(0x20).put<int8_t>(0);
		InPacket bag_recv(bag.bytes.data(), bag.bytes.size());
		auto one = storage_contents::read(bag_recv);

		require(!bag_recv.available(), "a bag's result is read");
		require(!one.meso && one.bags[4] && one.bags[4]->empty(), "the bag is carried empty");
		require(!one.bags[1] && !one.bags[2] && !one.bags[3] && !one.bags[5], "other bags are left as they were");

		Writer meso;
		meso.put<int8_t>(4).put<int64_t>(2).put<int32_t>(50);
		InPacket meso_recv(meso.bytes.data(), meso.bytes.size());
		auto mesos = storage_contents::read(meso_recv);

		require(!meso_recv.available() && mesos.meso == 50, "a meso result carries only the mesos");

		std::cout << "Storage contents passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
