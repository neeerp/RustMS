#pragma once

#include "../InPacket.h"
#include "../../Character/Inventory/ItemExpiry.h"

#include <array>
#include <cstdint>
#include <optional>
#include <string>
#include <vector>

namespace ms
{
	namespace storage_contents
	{
		struct Stored
		{
			int32_t itemid;
			int16_t count;
			int16_t flags = 0;
			// It has a Cash Shop serial.
			bool cash = false;
			int64_t expiration = item_expiry::BEFORE;
			// Who made or tagged it.
			std::string owner;
			// A pet's: when its Water of Life dries up, and the seconds one with limitedLife has left.
			int64_t date_dead = 0;
			int32_t remain_life = 0;
		};

		// What a TRUNK_RESULT carries. A bag it leaves out keeps its items,
		// and so do the mesos.
		struct Contents
		{
			uint8_t slots = 0;
			std::optional<int32_t> meso;
			// By inventory type, equip (1) to cash (5).
			std::array<std::optional<std::vector<Stored>>, 6> bags;
		};

		// One GW_ItemSlotBase without its position, by its type byte.
		inline Stored read_item(InPacket& recv)
		{
			int8_t type = recv.read_byte();
			Stored item = { recv.read_int(), 1 };
			item.cash = recv.read_bool();

			if (item.cash)
				recv.skip_long();

			item.expiration = recv.read_long();

			switch (type)
			{
			case 1:
				recv.skip(2 + 15 * 2); // upgrade slots, level, stats
				item.owner = recv.read_string();
				item.flags = recv.read_short();
				recv.skip(item.cash ? 10 : 18);
				recv.skip(12);
				break;
			case 3:
				recv.skip_padded_string(13);
				recv.skip(1 + 2 + 1); // level, closeness, fullness
				item.date_dead = recv.read_long();
				recv.skip(2 + 2); // attribute, skills
				item.remain_life = recv.read_int();
				recv.skip(2);
				break;
			default:
				item.count = recv.read_short();
				item.owner = recv.read_string();
				item.flags = recv.read_short();

				if (item.itemid / 10000 == 207 || item.itemid / 10000 == 233)
					recv.skip(8);
			}

			return item;
		}

		// The slots, then flags for what follows: 2 the mesos, 2 << type a
		// bag's count and items.
		inline Contents read(InPacket& recv)
		{
			Contents contents;
			contents.slots = static_cast<uint8_t>(recv.read_byte());
			int64_t flags = recv.read_long();

			if (flags & 2)
				contents.meso = recv.read_int();

			for (size_t type = 1; type < contents.bags.size(); type++)
			{
				if (!(flags & (int64_t{ 2 } << type)))
					continue;

				std::vector<Stored>& items = contents.bags[type].emplace();
				uint8_t count = static_cast<uint8_t>(recv.read_byte());

				for (uint8_t i = 0; i < count; i++)
					items.push_back(read_item(recv));
			}

			return contents;
		}
	}
}
