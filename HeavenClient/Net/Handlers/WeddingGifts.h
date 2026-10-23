#pragma once

#include "StorageContents.h"

#include <array>
#include <cstdint>
#include <string>
#include <vector>

namespace ms
{
	// What WEDDING_GIFT_RESULT carries (v83 0xa2a083, report b084).
	namespace wedding_gifts
	{
		// The gifts by inventory type, equip (1) to cash (5), in the order
		// the server lists them; a gift is taken by its type and its place
		// in that list.
		using Gifts = std::array<std::vector<storage_contents::Stored>, 6>;

		// A wishlist: a byte count, then the wishes.
		inline std::vector<std::string> read_wishlist(InPacket& recv)
		{
			std::vector<std::string> wishes;
			uint8_t count = static_cast<uint8_t>(recv.read_byte());

			for (uint8_t i = 0; i < count; i++)
				wishes.push_back(recv.read_string());

			return wishes;
		}

		// Flags, 2 << type for each type that follows as a count and its
		// items; a type left out keeps the gifts it had.
		inline void read_gifts(InPacket& recv, Gifts& gifts)
		{
			int64_t flags = recv.read_long();

			for (size_t type = 1; type < gifts.size(); type++)
			{
				if (!(flags & (int64_t{ 2 } << type)))
					continue;

				gifts[type].clear();
				uint8_t count = static_cast<uint8_t>(recv.read_byte());

				for (uint8_t i = 0; i < count; i++)
					gifts[type].push_back(storage_contents::read_item(recv));
			}
		}
	}
}
