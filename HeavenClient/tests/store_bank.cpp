#include "../Net/Handlers/StoreBank.h"
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

int main()
{
	try
	{
		using namespace ms;
		using namespace ms::store_bank;

		// After the npc: slots, the flags of the mesos, use and etc, then
		// those in order.
		Writer open;
		open.put<uint8_t>(16).put<int64_t>(2 | 2 << 2 | 2 << 4).put<int32_t>(100000);
		open.put<uint8_t>(1).stack(2000000, 1);
		open.put<uint8_t>(2).stack(4000000, 10).stack(4000001, 3);
		InPacket recv(open.bytes.data(), open.bytes.size());
		storage_contents::Contents contents = storage_contents::read(recv);
		std::vector<storage_contents::Stored> items = kept(contents);

		require(contents.meso == 100000, "the mesos");
		require(items.size() == 3, "every bag in one list");
		require(items[0].itemid == 2000000 && items[1].itemid == 4000000 && items[2].itemid == 4000001,
			"the use bag before the etc bag");
		require(items[1].count == 10, "a stack's count");
		require(kept(storage_contents::Contents{}).empty(), "nothing kept");

		require(!shows_count(1302000), "an equip shows no count");
		require(shows_count(2000000) && shows_count(3010000) && shows_count(4000000), "use, setup, etc");
		require(!shows_count(5000000), "a cash item shows no count");

		require(max_offset(0) == 0 && max_offset(5) == 0, "five rows don't scroll");
		require(max_offset(6) == 1 && max_offset(16) == 11, "more scroll a row at a time");

		require(fee_text(0, 0) == "Are you sure you want to retrieve them?", "no fee");
		require(fee_text(3, 4560) == "These were here for 3 days, so the storing\\nfee will cost you 3% of the total "
			"price,\\n4560 mesos.\\nAre you sure you want to retrieve them?", "three days");
		require(fee_text(200, 1000000).find("cost you 100% of") != std::string::npos, "at most all of it");

		require(message_text(RETRIEVED) == "You have retrieved your items and mesos.", "retrieved");
		require(message_text(0x21).find("lack of service fee") != std::string::npos, "no fee");
		require(message_text(0x22).find("full inventory") != std::string::npos, "inventory full");
		require(message_text(0x23).empty(), "an unknown code");

		require(elsewhere_text(NOTHING_KEPT, 0xff, "").rfind("I don't think you have", 0) == 0, "nothing kept");
		require(elsewhere_text(910000002, 0xfe, "2").rfind("I don't think you have", 0) == 0, "no channel");
		require(elsewhere_text(910000002, 1, "2").rfind("Your Personal Store is open #bin Channel 2, Free Market 2#k.",
			0) == 0, "open in channel 2, Free Market 2");
		require(elsewhere_text(910000003, 0, "Scania-1").find("Channel 1, Free Market 3#k") != std::string::npos,
			"the channel's number from its name");
		require(channel_number("Scania-12") == "12" && channel_number("7") == "7", "a channel's number");

		std::cout << "Store bank passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << error.what() << '\n';
		return 1;
	}
}
