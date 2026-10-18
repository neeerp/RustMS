#include "PartyCommands.h"

#include "ChatCommand.h"
#include "UI.h"

#include "UITypes/UIChatBar.h"

#include "../Gameplay/Stage.h"
#include "../Net/Packets/GameplayPackets.h"

namespace ms
{
	namespace PartyCommands
	{
		namespace
		{
			constexpr char NO_PARTY[] = "You have yet to join a party.";
			constexpr char NOT_LEADER[] = "You are not the master of the party.";
			constexpr char TOO_LOW[] = "Characters whose level is below Lv. 10, such as Beginners, Noblesses, and Legends cannot form a party.";
			// The party holds six.
			constexpr size_t CAPACITY = 6;

			void notice(const std::string& line)
			{
				if (auto chatbar = UI::get().get_element<UIChatBar>())
					chatbar->send_chatline(line, UIChatBar::LineType::SKILL);
			}

			// Beginners, Noblesses and Legends below level 10 can't form
			// or invite to a party.
			bool too_low(const Player& player)
			{
				const CharStats& stats = player.get_stats();
				uint16_t job = stats.get_job().get_id();
				bool beginner = job == 0 || job == 1000 || job == 2000 || job == 2001;

				return beginner && stats.get_stat(MapleStat::Id::LEVEL) < 10;
			}
		}

		bool run(const std::string& input)
		{
			using ChatCommand::same_text;
			using ChatCommand::with_argument;

			std::string name;

			if (same_text(input, "/partyinfo"))
				show_info();
			else if (same_text(input, "/makeparty"))
				create();
			else if (same_text(input, "/leaveparty"))
				leave();
			else if (with_argument(input, "/partyinvite", name))
				invite(name);
			else if (with_argument(input, "/partyexpel", name))
				expel(name);
			else
				return false;

			return true;
		}

		void show_info()
		{
			Player& player = Stage::get().get_player();
			const Party& party = player.get_party();

			if (!party.exists())
				return notice(NO_PARTY);

			std::string line = "Party member:";

			for (auto& member : party.get_members())
				line += " " + member.name;

			notice(line);

			if (party.is_leader(player.get_oid()))
				notice("You're the leader of the party.");
		}

		void create()
		{
			Player& player = Stage::get().get_player();

			if (player.get_party().exists())
				notice("Already have joined a party.");
			else if (too_low(player))
				notice(TOO_LOW);
			else
				CreatePartyPacket().dispatch();
		}

		void leave()
		{
			if (!Stage::get().get_player().get_party().exists())
				notice(NO_PARTY);
			else
				LeavePartyPacket().dispatch();
		}

		void invite(const std::string& name)
		{
			Player& player = Stage::get().get_player();
			const Party& party = player.get_party();

			if (too_low(player))
				notice(TOO_LOW);
			// v83's text has a leading tab, left out as in the party notices.
			else if (name == player.get_stats().get_name())
				notice("You may not invite yourself to the party.");
			else if (party.exists() && !party.is_leader(player.get_oid()))
				notice(NOT_LEADER);
			else if (party.exists() && party.get_members().size() >= CAPACITY)
				notice("It is impossible to invite someone to the party.");
			else
				InviteToPartyPacket(name).dispatch();
		}

		void expel(const std::string& name)
		{
			Player& player = Stage::get().get_player();
			const Party& party = player.get_party();

			if (!party.exists() || !party.is_leader(player.get_oid()))
				return notice(NOT_LEADER);

			if (const PartyMember* member = party.find(name))
				ExpelFromPartyPacket(member->id).dispatch();
			else
				notice("'" + name + "'is not a member of your party.");
		}
	}
}
