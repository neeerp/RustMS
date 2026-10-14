#include "BuddyHandlers.h"

#include "../Packets/GameplayPackets.h"

#include "../../Gameplay/Stage.h"
#include "../../IO/UI.h"
#include "../../IO/UITypes/UINotice.h"

namespace ms
{
	namespace
	{
		enum Operation : int8_t
		{
			LIST = 0x07,
			REQUEST = 0x09,
			OWN_LIST_FULL = 0x0B,
			THEIR_LIST_FULL = 0x0C,
			ALREADY_ADDED = 0x0D,
			NO_GAMEMASTERS = 0x0E,
			NOT_FOUND = 0x0F,
			CHANNEL = 0x14,
			CAPACITY = 0x15
		};

		constexpr uint16_t NAME_LENGTH = 13;
		constexpr uint16_t GROUP_LENGTH = 17;

		// The 39-byte GW_Friend record.
		Buddy read_buddy(InPacket& recv)
		{
			Buddy buddy;
			buddy.id = recv.read_int();
			buddy.name = recv.read_padded_string(NAME_LENGTH);
			recv.read_byte(); // Whether they are asking to be added
			buddy.channel = recv.read_int();
			buddy.group = recv.read_padded_string(GROUP_LENGTH);

			return buddy;
		}

		void notice(const std::string& message)
		{
			UI::get().emplace<UIOk>(message, [](bool) {});
		}
	}

	void BuddyListHandler::handle(InPacket& recv) const
	{
		BuddyList& buddies = Stage::get().get_player().get_buddies();
		int8_t op = recv.read_byte();

		switch (op)
		{
		case Operation::LIST:
		{
			uint8_t count = static_cast<uint8_t>(recv.read_byte());
			std::vector<Buddy> list;

			for (uint8_t i = 0; i < count; i++)
				list.push_back(read_buddy(recv));

			// Whether each is in the Cash Shop
			recv.skip(count * 4);

			buddies.update(std::move(list));
			break;
		}
		case Operation::REQUEST:
		{
			int32_t id = recv.read_int();
			std::string name = recv.read_string();

			UI::get().emplace<UIYesNo>(
				"'" + name + "' wants to add you as a buddy.\\nWould you like to accept?",
				[id](bool yes)
				{
					if (yes)
						AcceptBuddyPacket(id).dispatch();
					else
						DeleteBuddyPacket(id).dispatch();
				}
			);
			break;
		}
		case Operation::OWN_LIST_FULL:
			notice("Your buddy list is full.");
			break;
		case Operation::THEIR_LIST_FULL:
			notice("The user's buddy list is full.");
			break;
		case Operation::ALREADY_ADDED:
			notice("That character is already registered as your buddy.");
			break;
		case Operation::NO_GAMEMASTERS:
			notice("Gamemaster is not available as a buddy.");
			break;
		case Operation::NOT_FOUND:
			notice("That character is not registered.");
			break;
		case Operation::CHANNEL:
		{
			int32_t id = recv.read_int();
			recv.read_byte(); // In the Cash Shop
			buddies.set_channel(id, recv.read_int());
			break;
		}
		case Operation::CAPACITY:
			buddies.set_capacity(static_cast<uint8_t>(recv.read_byte()));
			break;
		default:
			break;
		}
	}
}
