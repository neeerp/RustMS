#include "GuildCommands.h"

#include "ChatCommand.h"
#include "UI.h"

#include "UITypes/UIChatBar.h"
#include "UITypes/UINotice.h"

#include "../Gameplay/Stage.h"
#include "../Net/Packets/GameplayPackets.h"

namespace ms
{
	namespace GuildCommands
	{
		namespace
		{
			constexpr char NOT_IN_GUILD[] = "You are not in the guild.";
			constexpr int32_t MASTER = 1;
			constexpr int32_t JR_MASTER = 2;

			void notice(const std::string& line)
			{
				if (auto chatbar = UI::get().get_element<UIChatBar>())
					chatbar->send_chatline(line, UIChatBar::LineType::SKILL);
			}
		}

		bool run(const std::string& input)
		{
			std::string name;

			if (ChatCommand::same_text(input, "/guildinfo"))
				show_info();
			else if (ChatCommand::same_text(input, "/leaveguild"))
				leave();
			else if (ChatCommand::with_argument(input, "/guildinvite", name))
				invite(name);
			else if (ChatCommand::with_argument(input, "/guildexpel", name))
				expel(name);
			else
				return false;

			return true;
		}

		void show_info()
		{
			const Player& player = Stage::get().get_player();
			const Guild& guild = player.get_guild();

			if (!guild.exists())
				return notice(NOT_IN_GUILD);

			// Everyone, online or not.
			std::string line = "Guild Member:";

			for (auto& member : guild.get_data().members)
				line += " " + member.name;

			notice(line);

			if (guild.is_master(player.get_oid()))
				notice("You are the guild master.");
		}

		void leave()
		{
			const Player& player = Stage::get().get_player();
			const Guild& guild = player.get_guild();

			if (!guild.exists())
				notice(NOT_IN_GUILD);
			else if (guild.is_master(player.get_oid()))
				notice("You cannot quit the guild, since you are the master of the guild.");
			else
				UI::get().emplace<UIYesNo>("Would you like to quit the guild?", [](bool yes)
				{
					const Player& player = Stage::get().get_player();

					if (yes)
						LeaveGuildPacket(player.get_oid(), player.get_stats().get_name()).dispatch();
				});
		}

		void invite(const std::string& name)
		{
			const Player& player = Stage::get().get_player();
			const Guild& guild = player.get_guild();

			if (name == player.get_stats().get_name())
				notice("You cannot invite yourself to the guild.");
			else if (!guild.exists())
				notice(NOT_IN_GUILD);
			else if (!can_invite(player))
				notice("You are not the master of the guild.");
			else if (guild.full())
				notice("You cannot invite more people to the guild.");
			else
				InviteToGuildPacket(name).dispatch();
		}

		// Unlike the guild tab, the command leaves it to the server whether
		// the player may expel at all.
		void expel(const std::string& name)
		{
			const Player& player = Stage::get().get_player();
			const Guild& guild = player.get_guild();

			if (!guild.exists())
				return notice(NOT_IN_GUILD);

			const GuildMember* member = guild.find_by_name(name);

			if (!member)
				notice("'" + name + "' is not the member of the guild.");
			else if (guild.rank(player.get_oid()) == JR_MASTER && member->rank == JR_MASTER)
				notice("Only Guild Master can expel Jr.Masters.");
			else if (member->rank == MASTER)
				notice("You may not ban the Guild Master.");
			else
				UI::get().emplace<UIYesNo>("Expel '" + member->name + "' from the guild?",
					[cid = member->id, target = member->name](bool yes)
					{
						if (yes)
							ExpelFromGuildPacket(cid, target).dispatch();
					});
		}

		bool can_invite(const Player& player)
		{
			const Guild& guild = player.get_guild();
			int32_t self = player.get_oid();

			return guild.exists() && (guild.is_master(self) || guild.rank(self) == JR_MASTER);
		}
	}
}
