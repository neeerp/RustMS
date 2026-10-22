#pragma once

#include "../../InPacket.h"

#include <string>
#include <vector>

namespace ms
{
	// A pet as the field shows it.
	struct PetEntry
	{
		int32_t itemid;
		std::string name;
		// The cash serial of its item.
		int64_t serial;
		Point<int16_t> position;
		uint8_t stance;
		int16_t fh;
		// A label ring is worn for it, so it shows its kind's name tag.
		bool name_tag = false;
		// A quote ring is worn for it.
		bool chat_balloon = false;
	};

	namespace PetParser
	{
		// CPet::Init (FUN_00703314).
		inline PetEntry parse_pet(InPacket &recv)
		{
			PetEntry pet;
			pet.itemid = recv.read_int();
			pet.name = recv.read_string();
			pet.serial = recv.read_long();
			pet.position = recv.read_point();
			pet.stance = recv.read_byte();
			pet.fh = recv.read_short();
			pet.name_tag = recv.read_bool();
			pet.chat_balloon = recv.read_bool();

			return pet;
		}

		// A player's pets in SPAWN_PLAYER, in list order: each follows a
		// 1, and a 0 ends them (FUN_0097f55d).
		inline std::vector<PetEntry> parse_pets(InPacket &recv)
		{
			std::vector<PetEntry> pets;

			while (recv.read_byte())
				pets.push_back(parse_pet(recv));

			return pets;
		}
	}
}
