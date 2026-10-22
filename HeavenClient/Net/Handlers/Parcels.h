#pragma once

#include "StorageContents.h"

#include <cstdint>
#include <optional>
#include <string>
#include <vector>

// Duey's parcels as PARCEL (322) lists them (report b028).
namespace ms::parcels
{
	// The window keeps no more (FUN_006f0b72).
	constexpr size_t LIMIT = 10;
	constexpr int64_t TICKS_PER_DAY = 864000000000;

	struct Parcel
	{
		int32_t id = 0;
		std::string sender;
		int32_t meso = 0;
		// When it expires, as a FILETIME.
		int64_t expires = 0;
		std::optional<std::string> message;
		bool quick = false;
		std::optional<storage_contents::Stored> item;
	};

	// A fixed-length string ends at its first NUL.
	inline std::string read_fixed(InPacket& recv, uint16_t length)
	{
		std::string text;
		bool ended = false;

		for (uint16_t i = 0; i < length; i++)
		{
			char letter = recv.read_byte();

			if (letter == '\0')
				ended = true;
			else if (!ended)
				text.push_back(letter);
		}

		return text;
	}

	// One package record (FUN_004e4345): 0xea bytes, then the item.
	inline Parcel read(InPacket& recv)
	{
		Parcel parcel;
		parcel.id = recv.read_int();
		parcel.sender = read_fixed(recv, 13);
		parcel.meso = recv.read_int();
		parcel.expires = recv.read_long();
		bool has_message = recv.read_int() != 0;
		std::string message = read_fixed(recv, 200);

		if (has_message)
			parcel.message = message;

		parcel.quick = recv.read_bool();

		if (recv.read_bool())
			parcel.item = storage_contents::read_item(recv);

		return parcel;
	}

	struct Lists
	{
		std::vector<Parcel> waiting;
		std::vector<Parcel> expired;
	};

	// Mode 8's two lists: those waiting, and those that expired since.
	inline Lists read_lists(InPacket& recv)
	{
		Lists lists;

		for (auto list : { &lists.waiting, &lists.expired })
		{
			uint8_t count = static_cast<uint8_t>(recv.read_byte());

			for (uint8_t i = 0; i < count; i++)
				list->push_back(read(recv));
		}

		return lists;
	}

	// Whole days until a parcel expires, rounded down (FUN_006efff3).
	inline int64_t days_left(int64_t expires, int64_t now)
	{
		int64_t left = expires - now;
		int64_t days = left / TICKS_PER_DAY;

		return left % TICKS_PER_DAY < 0 ? days - 1 : days;
	}

	// 30 days or more left: it's still on its way, and can't be claimed.
	inline bool delivering(int64_t expires, int64_t now)
	{
		return days_left(expires, now) >= 30;
	}

	// The row's status (strings 0xf29 to 0xf2b).
	inline std::string status(int64_t expires, int64_t now)
	{
		int64_t days = days_left(expires, now);

		if (days >= 30)
			return "Delivering";

		if (days > 0)
			return std::to_string(days) + " day(s)";

		return "Will expire soon";
	}

	// Equipment, throwing stars and bullets go whole: v83 asks no count for
	// them, and shows none (FUN_004284be).
	inline bool single(int32_t item_id)
	{
		int32_t kind = item_id / 10000;

		return item_id / 1000000 == 1 || kind == 207 || kind == 233;
	}

	// A normal send's fee on top of its tax.
	constexpr int32_t SEND_FEE = 5000;
	// No send takes more mesos.
	constexpr int32_t MOST_MESOS = 100000000;
	// Players of this level or under may send at most a million.
	constexpr int16_t LOW_LEVEL = 15;
	constexpr int32_t LOW_LEVEL_MESOS = 1000000;

	// The tax on the mesos sent (FUN_006eedfe), in v83's doubles, which put
	// some 1.8% amounts one below the server's.
	inline int32_t tax(int32_t meso)
	{
		double rate = meso >= 100000000 ? 0.06
			: meso >= 25000000 ? 0.05
			: meso >= 10000000 ? 0.04
			: meso >= 5000000 ? 0.03
			: meso >= 1000000 ? 0.018
			: meso >= 100000 ? 0.008
			: 0.0;

		return static_cast<int32_t>(meso * rate);
	}

	// The most the mesos prompt offers out of `left`, the purse less a
	// normal send's fee (FUN_006f3875, FUN_006f20a5). v83 takes off its
	// tax roughly: the first of these rates whose result lands in its
	// range, or nothing.
	inline int32_t most_mesos(int32_t left)
	{
		struct Tier
		{
			double rate;
			double low;
			double high;
		};

		constexpr Tier tiers[] = {
			{ 0.9615384615384615, 1e7, 1e300 },
			{ 0.970873786407767, 5e6, 1e7 },
			{ 0.9803921568627451, 1e6, 5e6 },
			{ 0.9900990099009901, 1e5, 1e6 },
			{ 0.9950248756218907, 5e4, 1e5 }
		};

		int32_t most = left;

		for (const Tier& tier : tiers)
		{
			double kept = left * tier.rate;

			if (kept >= tier.low && kept < tier.high)
			{
				most = static_cast<int32_t>(kept);
				break;
			}
		}

		return most < MOST_MESOS ? most : MOST_MESOS;
	}

	// What the notice of a parcel's arrival says (FUN_0052065d, type 10):
	// two lines, or one when the second is empty. Only mode 0x19 names the
	// sender. v83 draws each string on one line: "Package Received." alone
	// sits centred, and the name goes where its second line would be.
	struct Arrival
	{
		std::string first, second;
	};

	inline Arrival arrival(const std::string& sender, bool quick)
	{
		if (sender.empty())
			return quick ? Arrival{ "Quick delivey/Package", "Arrived!" } : Arrival{ "Package Received.", "" };

		return { quick ? "The Quick Delivery has arrived!" : "Package Received.", "[" + sender + "]" };
	}

	// What a result code says (FUN_006f5be2), with UIOk's line breaks; some
	// say nothing.
	inline std::optional<std::string> result_text(uint8_t mode)
	{
		switch (mode)
		{
			case 0x0a:
				return "You do not have enough mesos.";
			case 0x0b:
				return "You have made an incorrect request.";
			case 0x0c:
				return "Please double-check the receiver's name.";
			case 0x0d:
				return "You may not send the package to a character \\nunder the same account.";
			case 0x0e:
				return "The receiver's package storage is full.";
			case 0x0f:
				return "The character is unable to receive packages.";
			case 0x10:
				return "Unable to send the one-of-a-kind item\\n due to the recipient already possessing one\\n at "
					"the Quick Delivery Inbox.";
			case 0x11:
				return "Players that are Level 15 and below \\nmay only trade 1 million mesos per day. \\nYou have "
					"reached the limit today,\\nplease try again tomorrow.";
			case 0x12:
				return "The package has been successfully sent.";
			case 0x13:
			case 0x1c:
				return "An unknown error has occurred.";
			case 0x15:
				return "The inventory does not have enough free slots available.\\nPlease double check and try again.";
			case 0x16:
				return "Unable to retrieve mesos & items \\ndue to the item being only one of a kind.";
			default:
				return std::nullopt;
		}
	}
}
