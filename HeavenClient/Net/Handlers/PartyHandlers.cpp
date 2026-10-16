#include "PartyHandlers.h"

#include "../Packets/GameplayPackets.h"
#include "../Packets/MessagingPackets.h"

#include "../../Configuration.h"

#include "../../Gameplay/Stage.h"
#include "../../IO/UI.h"
#include "../../IO/UITypes/UIChatBar.h"
#include "../../IO/UITypes/UINotice.h"

namespace ms
{
	namespace
	{
		enum Operation : int8_t
		{
			INVITE = 0x04,
			SILENT_UPDATE = 0x07,
			CREATED = 0x08,
			LEAVE = 0x0C,
			JOIN = 0x0F,
			CHANGE_LEADER = 0x1B,
			// No message; doors and the like, which the client lacks.
			QUIET = 0x12,
			DOOR_TOWN = 0x22,
			DOOR_FIELD = 0x23,
			DOOR_REMOVE = 0x25,
			// Messages, some carrying text.
			ALREADY_JOINED = 0x09,
			BEGINNER = 0x0A,
			NOT_JOINED = 0x0D,
			ALREADY_IN_PARTY = 0x10,
			FULL = 0x11,
			NOT_FOUND = 0x13,
			BLOCKING = 0x15,
			BUSY = 0x16,
			DECLINED = 0x17,
			KICK_MAP = 0x19,
			LEADER_NOT_NEAR = 0x1C,
			NOBODY_NEAR = 0x1D,
			OTHER_CHANNEL = 0x1E,
			GM = 0x20,
			NO_CHARACTER = 0x21,
			SERVER_NOTICE = 0x24
		};

		constexpr char UNEXPECTED[] = "Your request for a party didn't work due to an unexpected error.";

		constexpr size_t CAPACITY = 6;
		constexpr uint16_t NAME_LENGTH = 13;

		// v83 logs every party notice in chat type 0xc; party chat has its own.
		void party_line(const std::string& line, UIChatBar::LineType type = UIChatBar::LineType::SKILL)
		{
			if (auto chatbar = UI::get().get_element<UIChatBar>())
				chatbar->send_chatline(line, type);
		}

		// The member block: six slots of each column, empty slots zeroed.
		// Returns the leader.
		int32_t read_members(InPacket& recv, std::vector<PartyMember>& members)
		{
			PartyMember slots[CAPACITY];

			for (auto& slot : slots)
				slot.id = recv.read_int();

			for (auto& slot : slots)
				slot.name = recv.read_padded_string(NAME_LENGTH);

			for (auto& slot : slots)
				slot.job = static_cast<int16_t>(recv.read_int());

			for (auto& slot : slots)
				slot.level = static_cast<int16_t>(recv.read_int());

			for (auto& slot : slots)
				slot.channel = recv.read_int();

			int32_t leader = recv.read_int();

			for (auto& slot : slots)
				slot.map_id = recv.read_int();

			// Mystic doors
			recv.skip(CAPACITY * 16);

			for (auto& slot : slots)
				if (slot.id != 0)
					members.push_back(slot);

			return leader;
		}

		void read_party(InPacket& recv, int32_t party_id)
		{
			std::vector<PartyMember> members;
			int32_t leader = read_members(recv, members);
			Stage::get().get_player().get_party().update(party_id, leader, std::move(members));
		}
	}

	void PartyOperationHandler::handle(InPacket& recv) const
	{
		Player& player = Stage::get().get_player();
		Party& party = player.get_party();
		int32_t self = player.get_oid();
		int8_t op = recv.read_byte();

		switch (op)
		{
		case Operation::INVITE:
		{
			int32_t party_id = recv.read_int();
			std::string inviter = recv.read_string();

			UI::get().emplace<UIYesNo>(
				"'" + inviter + "' has invited you to a party.\\nWould you like to join?",
				[party_id, inviter](bool yes)
				{
					if (yes)
						JoinPartyPacket(party_id).dispatch();
					else
						DenyPartyRequestPacket(inviter).dispatch();
				}
			);
			break;
		}
		case Operation::SILENT_UPDATE:
		{
			int32_t party_id = recv.read_int();
			read_party(recv, party_id);
			break;
		}
		case Operation::CREATED:
		{
			int32_t party_id = recv.read_int();

			PartyMember leader;
			leader.id = self;
			leader.name = player.get_stats().get_name();
			leader.job = player.get_stats().get_stat(MapleStat::Id::JOB);
			leader.level = player.get_level();
			leader.channel = Configuration::get().get_channelid();
			leader.map_id = Stage::get().get_mapid();
			party.update(party_id, self, { leader });

			party_line("You have created a new party.");
			break;
		}
		case Operation::LEAVE:
		{
			int32_t party_id = recv.read_int();
			int32_t member_id = recv.read_int();
			bool disbanded = !recv.read_bool();

			if (disbanded)
			{
				bool led = party.is_leader(self);
				party.clear();

				if (led)
					party_line("You have quit as the leader of the party. The party has been disbanded.");
				else
					party_line("You have left the party since the party leader quit.");

				break;
			}

			bool expelled = recv.read_bool();
			std::string name = recv.read_string();

			if (member_id == self)
			{
				party.clear();
				party_line(expelled ? "You have been expelled from the party." : "You have left the party.");
			}
			else
			{
				read_party(recv, party_id);
				party_line("'" + name + (expelled ? "' have been expelled from the party." : "' have left the party."));
			}

			break;
		}
		case Operation::JOIN:
		{
			int32_t party_id = recv.read_int();
			std::string name = recv.read_string();
			read_party(recv, party_id);

			if (name == player.get_stats().get_name())
				party_line("You have joined the party.");
			else
				party_line("'" + name + "' has joined the party.");

			break;
		}
		case Operation::ALREADY_JOINED:
		case Operation::ALREADY_IN_PARTY:
			party_line("Already have joined a party.");
			break;
		case Operation::BEGINNER:
			party_line("A beginner can't create a party.");
			break;
		case Operation::NOT_JOINED:
			party_line("You have yet to join a party.");
			break;
		case Operation::FULL:
			party_line("The party you're trying to join is already in full capacity.");
			break;
		case Operation::NOT_FOUND:
			party_line("Unable to find the requested character in this channel.");
			break;
		case Operation::BLOCKING:
			party_line(recv.read_string() + " is currently blocking any party invitations.");
			break;
		case Operation::BUSY:
			party_line("'" + recv.read_string() + "' is taking care of another invitation.");
			break;
		case Operation::DECLINED:
			party_line(recv.read_string() + " have denied request to the party.");
			break;
		case Operation::KICK_MAP:
			party_line("Cannot kick another user in this map");
			break;
		case Operation::LEADER_NOT_NEAR:
			party_line("This can only be given to a party member within the vicinity.");
			break;
		case Operation::NOBODY_NEAR:
			party_line("Unable to hand over the leadership post; No party member is currently within the vicinity of the party leader.");
			break;
		case Operation::OTHER_CHANNEL:
			party_line("You may only change with the party member that's on the same channel.");
			break;
		case Operation::GM:
			party_line("As a GM, you're forbidden from creating a party.");
			break;
		case Operation::NO_CHARACTER:
			party_line("Unable to find the character.");
			break;
		case Operation::SERVER_NOTICE:
			party_line(recv.read_bool() ? recv.read_string() : UNEXPECTED);
			break;
		case Operation::CHANGE_LEADER:
		{
			int32_t leader = recv.read_int();
			bool disconnected = recv.read_bool();
			party.set_leader(leader);

			if (auto member = party.find(leader))
			{
				if (disconnected)
					party_line("Due to the party leader disconnecting from the game, " + member->name + " has been assigned as the new leader.");
				else
					party_line(member->name + " has become the leader of the party.");
			}

			break;
		}
		case Operation::QUIET:
		case Operation::DOOR_TOWN:
		case Operation::DOOR_FIELD:
		case Operation::DOOR_REMOVE:
			break;
		default:
			party_line(UNEXPECTED);
			break;
		}
	}

	void UpdatePartyMemberHpHandler::handle(InPacket& recv) const
	{
		int32_t cid = recv.read_int();
		int32_t hp = recv.read_int();
		int32_t maxhp = recv.read_int();

		Stage::get().get_player().get_party().set_hp(cid, hp, maxhp);
	}

	void MultiChatHandler::handle(InPacket& recv) const
	{
		int8_t mode = recv.read_byte();
		std::string name = recv.read_string();
		std::string text = recv.read_string();

		if (mode == MultiChatPacket::Mode::PARTY)
			party_line(name + " : " + text, UIChatBar::LineType::PARTY);
		else if (mode == MultiChatPacket::Mode::BUDDY)
			party_line(name + " : " + text, UIChatBar::LineType::BUDDY);
	}
}
