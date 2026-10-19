#include "BuddyHandlers.h"

#include "../Packets/GameplayPackets.h"

#include "../../Configuration.h"
#include "../../Gameplay/Stage.h"
#include "../../IO/BuddyCommands.h"
#include "../../IO/UI.h"
#include "../../IO/UITypes/UIInviteCard.h"
#include "../../IO/UITypes/UINotice.h"

namespace ms
{
	namespace
	{
		enum Operation : int8_t
		{
			LIST = 0x07,
			UPDATE = 0x08,
			REQUEST = 0x09,
			LIST_AGAIN = 0x0A,
			OWN_LIST_FULL = 0x0B,
			THEIR_LIST_FULL = 0x0C,
			ALREADY_ADDED = 0x0D,
			NO_GAMEMASTERS = 0x0E,
			NOT_FOUND = 0x0F,
			RESULT = 0x10,
			RESULT_2 = 0x11,
			LIST_ONCE_MORE = 0x12,
			RESULT_3 = 0x13,
			CHANNEL = 0x14,
			CAPACITY = 0x15,
			RESULT_4 = 0x16
		};

		constexpr uint16_t NAME_LENGTH = 13;
		constexpr uint16_t GROUP_LENGTH = 17;

		// The 39-byte GW_Friend record.
		Buddy read_buddy(InPacket& recv)
		{
			Buddy buddy;
			buddy.id = recv.read_int();
			buddy.name = recv.read_padded_string(NAME_LENGTH);
			buddy.flag = static_cast<uint8_t>(recv.read_byte());
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
		// v83 reads all three the same way (FUN_00a3ff8b).
		case Operation::LIST:
		case Operation::LIST_AGAIN:
		case Operation::LIST_ONCE_MORE:
		{
			uint8_t count = static_cast<uint8_t>(recv.read_byte());
			std::vector<Buddy> list;

			for (uint8_t i = 0; i < count; i++)
				list.push_back(read_buddy(recv));

			for (Buddy& buddy : list)
				buddy.in_cash_shop = recv.read_int() != 0;

			buddies.update(std::move(list));
			break;
		}
		// Changes a listed buddy and adds nobody (FUN_00a400ad).
		case Operation::UPDATE:
		{
			int32_t id = recv.read_int();
			Buddy buddy = read_buddy(recv);
			buddy.in_cash_shop = recv.read_bool();
			buddies.replace(id, std::move(buddy));
			break;
		}
		// The request joins the list, hidden, so that accepting it works
		// and it can be asked again after a map change (FUN_00a40028).
		case Operation::REQUEST:
		{
			int32_t id = recv.read_int();
			std::string name = recv.read_string();
			Buddy buddy = read_buddy(recv);
			buddy.in_cash_shop = recv.read_bool();
			buddies.add(std::move(buddy));

			BuddyCommands::ask(name, id);
			break;
		}
		case Operation::OWN_LIST_FULL:
			notice("Your buddy list is full.");
			break;
		case Operation::THEIR_LIST_FULL:
			notice("The user's buddy list is full");
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
		// A buddy logging in or moving to the player's channel shows a card,
		// unless they are in the player's guild, which tells them itself.
		case Operation::CHANNEL:
		{
			int32_t id = recv.read_int();
			bool in_cash_shop = recv.read_bool();
			int32_t channel = recv.read_int();
			BuddyList::Arrival arrival = buddies.set_channel(id, channel, in_cash_shop, Configuration::get().get_channelid());

			if (arrival == BuddyList::Arrival::NONE)
				break;

			const Buddy* buddy = buddies.find(id);
			auto cards = UI::get().get_element<UIInviteCard>();

			if (!buddy || !cards)
				break;

			if (arrival == BuddyList::Arrival::SAME_CHANNEL)
				cards->notify(buddy->name, true);
			else if (!Stage::get().get_player().get_guild().find_by_name(buddy->name))
				cards->notify(buddy->name, false);

			break;
		}
		// The server's own words, if any (FUN_00428211).
		case Operation::RESULT:
		case Operation::RESULT_2:
		case Operation::RESULT_3:
		case Operation::RESULT_4:
			if (recv.read_bool())
				notice(recv.read_string());
			else
				notice("The request was denied due to an unknown error.");

			break;
		case Operation::CAPACITY:
			buddies.set_capacity(static_cast<uint8_t>(recv.read_byte()));
			break;
		default:
			break;
		}
	}
}
