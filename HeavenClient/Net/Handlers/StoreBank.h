#pragma once

#include "StorageContents.h"

#include <algorithm>
#include <cstdint>
#include <cstdio>
#include <string>
#include <vector>

namespace ms
{
	// The parts of v83's Store Bank (Fredrick's CStoreBankDlg, 0x79a0ff)
	// that need no UI: where a hired merchant's owner takes back what it
	// kept when it closed without them.
	namespace store_bank
	{
		// STORE_BANK modes (0x79b3b6).
		enum Mode : int8_t
		{
			OPEN = 0x23,
			FEE = 0x24,
			ELSEWHERE = 0x25,
			UNAVAILABLE = 0x26
		};

		// The answers to a Get All confirmed (STORE_BANK_MESSAGE, 0x79b70e).
		constexpr int8_t RETRIEVED = 0x1e;

		// The window's requests (STORE_BANK_REQUEST).
		enum Request : int8_t
		{
			GET_ALL = 0x1a,
			CONFIRM = 0x1b,
			CLOSE = 0x1c
		};

		constexpr int16_t ROWS = 5;

		// A mode 0x25 with this map, or a channel past 0xfd, says Fredrick
		// keeps nothing.
		constexpr int32_t NOTHING_KEPT = 999999999;

		const std::string UNAVAILABLE_TEXT = "This is currently unavailable.\\nPlease try again later";

		// What the bank keeps, in the order the server lists it: equips
		// first, cash last.
		inline std::vector<storage_contents::Stored> kept(const storage_contents::Contents &contents)
		{
			std::vector<storage_contents::Stored> items;

			for (const auto &bag : contents.bags)
				if (bag)
					items.insert(items.end(), bag->begin(), bag->end());

			return items;
		}

		// Use, setup and etc items show how many there are.
		inline bool shows_count(int32_t itemid)
		{
			int32_t category = itemid / 1000000;

			return category >= 2 && category <= 4;
		}

		// The first row the list may scroll to.
		inline int16_t max_offset(size_t items)
		{
			return static_cast<int16_t>(std::max<int64_t>(0, static_cast<int64_t>(items) - ROWS));
		}

		// The question before taking it all (SP 0xda2, 0xda3): the fee is
		// a percent a day, all of it after 100 days.
		inline std::string fee_text(int32_t days, int32_t fee)
		{
			if (fee == 0)
				return "Are you sure you want to retrieve them?";

			char text[256];
			std::snprintf(text, sizeof(text), "These were here for %d days, so the storing\\nfee will cost you %d%% of the "
				"total price,\\n%d mesos.\\nAre you sure you want to retrieve them?", days, std::min(days, 100), fee);

			return text;
		}

		// The notice answering a Get All confirmed (SP 0xda4 to 0xda8).
		inline std::string message_text(int8_t code)
		{
			switch (code)
			{
			case RETRIEVED:
				return "You have retrieved your items and mesos.";
			case 0x1f:
				return "Unable to retrieve mesos and items due to\\ntoo much money stored\\nat the Store Bank.";
			case 0x20:
				return "Unable to retrieve mesos and items due to\\none of the items\\nthat can only be possessed one "
					"at a time.";
			case 0x21:
				return "Due to the lack of service fee, you were unable to \\nretrieve mesos or items.";
			case 0x22:
				return "Unable to retrieve mesos and items\\ndue to full inventory.";
			default:
				return "";
			}
		}

		// The channel in a name such as "Scania-1": what follows the
		// first '-' (v83 0x988557), or all of it.
		inline std::string channel_number(const std::string &channel_name)
		{
			size_t dash = channel_name.find('-');

			return dash == std::string::npos ? channel_name : channel_name.substr(dash + 1);
		}

		// What Fredrick says when he keeps nothing, or the merchant is
		// open (SP 0xd9f, 0xda0): the channel's number, and the Free Market
		// room the map id ends in.
		inline std::string elsewhere_text(int32_t mapid, uint8_t channel, const std::string &channel_name)
		{
			if (mapid == NOTHING_KEPT || channel > 0xfd)
				return "I don't think you have any items or money to retrieve here.\r\nThis is where you retrieve the "
					"items and mesos that you couldn't get from your Hired Merchant. You'll also need to see me as "
					"the character that opened the Personal Store.";

			return "Your Personal Store is open #bin Channel " + channel_number(channel_name) + ", Free Market " + std::to_string(mapid % 100) +
				"#k.\r\nIf you need me, then please close your personal store first before seeing me.";
		}
	}
}
