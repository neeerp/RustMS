#pragma once

#include <string>

namespace ms
{
	class Player;

	// v83's guild commands, with the checks the client makes before asking
	// the server. The guild tab's buttons and the context menu share them.
	namespace GuildCommands
	{
		// Runs input if it is a guild command; returns whether it was. They
		// are matched as the party commands are.
		bool run(const std::string& input);

		// /guildinfo (FUN_00a39033)
		void show_info();
		// /leaveguild (FUN_005308e0)
		void leave();
		// /guildinvite (FUN_005306d5)
		void invite(const std::string& name);
		// /guildexpel (FUN_00530a0d)
		void expel(const std::string& name);

		// The guild master and the Jr. Masters invite (FUN_0053088f).
		bool can_invite(const Player& player);
	}
}
