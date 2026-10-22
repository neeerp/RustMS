#include "../Net/Handlers/Parcels.h"
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

	Writer& fixed(const std::string& text, size_t length)
	{
		bytes.insert(bytes.end(), text.begin(), text.end());
		bytes.insert(bytes.end(), length - text.size(), 0);
		return *this;
	}

	// One package record, as RustMS and v83 write it.
	Writer& parcel(int32_t id, const std::string& sender, int32_t meso, int64_t expires, const std::string* message,
		bool item)
	{
		put(id).fixed(sender, 13).put(meso).put(expires).put<int32_t>(message ? 1 : 0);
		fixed(message ? *message : "", 200).put<int8_t>(message ? 1 : 0).put<int8_t>(item);

		if (item)
		{
			// A use item: type 2, id, no cash, no expiry, 30 of them, no
			// owner, no flags.
			put<int8_t>(2).put<int32_t>(2000000).put<int8_t>(0).put<int64_t>(ms::item_expiry::BEFORE);
			put<int16_t>(30).put<int16_t>(0).put<int16_t>(0);
		}

		return *this;
	}
};

int main()
{
	try
	{
		using namespace ms;
		using namespace ms::parcels;

		int64_t now = item_expiry::UNIX_EPOCH + 1792505109LL * item_expiry::TICKS_PER_SECOND;
		int64_t day = TICKS_PER_DAY;

		require(days_left(now + 30 * day, now) == 30, "whole days are counted");
		require(days_left(now + 29 * day + 1, now) == 29, "and rounded down");
		require(days_left(now - 1, now) == -1, "below zero too");
		require(delivering(now + 30 * day, now) && !delivering(now + 30 * day - 1, now),
			"30 days or more is still on its way");
		require(status(now + 30 * day, now) == "Delivering", "shown as Delivering");
		require(status(now + 29 * day + 5, now) == "29 day(s)", "then by the days left");
		require(status(now + day, now) == "1 day(s)", "down to one");
		require(status(now + day - 1, now) == "Will expire soon", "under a day, it expires soon");
		require(status(now - day, now) == "Will expire soon", "and so when overdue");

		std::string note = "for you";
		Writer w;
		w.put<uint8_t>(2);
		w.parcel(7, "Alice", 200000, now + 29 * day, &note, false);
		w.parcel(8, "Bob", 0, now + 30 * day, nullptr, true);
		w.put<uint8_t>(1);
		w.parcel(3, "Carol", 50, now - day, nullptr, false);
		w.put<int8_t>(0x55);

		InPacket recv(w.bytes.data(), w.bytes.size());
		Lists lists = read_lists(recv);

		require(lists.waiting.size() == 2 && lists.expired.size() == 1, "two waiting, one expired");
		const Parcel& first = lists.waiting[0];
		require(first.id == 7 && first.sender == "Alice" && first.meso == 200000, "the id, sender, and mesos");
		require(first.expires == now + 29 * day, "the expiry");
		require(first.message && *first.message == "for you" && first.quick, "a quick parcel's message");
		require(!first.item, "no item");
		const Parcel& second = lists.waiting[1];
		require(!second.message && !second.quick, "a normal parcel has no message");
		require(second.item && second.item->itemid == 2000000 && second.item->count == 30, "its item");
		require(lists.expired[0].sender == "Carol" && lists.expired[0].meso == 50, "the expired list");
		require(recv.read_byte() == 0x55, "the lists end where they should");

		// Text after a NUL is padding.
		Writer junk;
		junk.put<int32_t>(1).fixed(std::string("Ann\0zzz", 7), 13).put<int32_t>(0).put<int64_t>(now);
		junk.put<int32_t>(1).fixed(std::string("hi\0there", 8), 200).put<int8_t>(0).put<int8_t>(0);
		InPacket padded(junk.bytes.data(), junk.bytes.size());
		Parcel ann = read(padded);
		require(ann.sender == "Ann" && ann.message && *ann.message == "hi", "a fixed string ends at its NUL");

		// Equips, stars and bullets go whole.
		require(single(1302000) && single(2070000) && single(2330000), "equips and rechargeables go whole");
		require(!single(2000000) && !single(4000000) && !single(2080000), "other stacks are counted");

		// The tax, by v83's doubles.
		require(tax(99999) == 0, "nothing under 100,000");
		require(tax(100000) == 800 && tax(999999) == 7999, "0.8% under a million");
		require(tax(1000000) == 18000, "1.8% from a million");
		require(tax(1337500) == 24074, "where the double falls short, one below the exact 24075");
		require(tax(5000000) == 150000 && tax(10000000) == 400000, "3% and 4%");
		require(tax(25000000) == 1250000 && tax(100000000) == 6000000, "5% and 6%");

		// The mesos prompt's ceiling.
		require(most_mesos(40000) == 40000, "small purses keep everything");
		require(most_mesos(100000) == 99502, "100,000 less half a percent");
		require(most_mesos(1000000) == 990099, "a million less 1%");
		require(most_mesos(110000000) == 100000000, "capped at 100 million");
		require(most_mesos(100000000) == 96153846, "100 million less 4%, all v83 takes");

		require(result_text(0x12) && *result_text(0x12) == "The package has been successfully sent.", "sent");
		require(result_text(0x1c) == result_text(0x13), "0x1c is the unknown error too");
		require(!result_text(0x09) && !result_text(0x14), "some codes say nothing");

		std::cout << "parcels tests passed\n";
		return 0;
	}
	catch (const std::exception& error)
	{
		std::cerr << "parcels test failed: " << error.what() << "\n";
		return 1;
	}
}
