#pragma once

#include <string>

namespace ms
{
	// v83's GM slash commands, for an account whose grade bytes allow them.
	namespace GmCommands
	{
		// Runs input if it is a GM command the account may use, and logs
		// it to the server (ADMIN_LOG); returns whether it was one.
		bool run(const std::string& input);
	}
}
