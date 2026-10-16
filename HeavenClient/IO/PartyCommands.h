#pragma once

#include <string>

namespace ms
{
	// v83's party commands (FUN_0052c40c), with the checks the client makes
	// before asking the server. The party window's buttons share them.
	namespace PartyCommands
	{
		// Runs input if it is a party command; returns whether it was. As in
		// v83, a command is matched without regard to case, and a name is
		// whatever follows the command and one space.
		bool run(const std::string& input);

		// /partyinfo (FUN_00a3f6fd)
		void show_info();
		// /makeparty (FUN_0052fce1)
		void create();
		// /leaveparty (FUN_0052fe00)
		void leave();
		// /partyinvite (FUN_0052fecf)
		void invite(const std::string& name);
		// /partyexpel (FUN_005300c1)
		void expel(const std::string& name);
	}
}
