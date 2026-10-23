#pragma once

#include <cstdint>
#include <map>
#include <string>
#include <vector>

namespace ms
{
	// The script an NPC runs when talked to, chosen by date (v83: the
	// CNpcTemplate list at +0x7c). Its Npc/<id>.img/info/script/<n> windows
	// come first, then those SET_NPC_SCRIPTABLE (0x107, FUN_006deeb7) adds.
	// The click asks the server to talk only when there is one (FUN_006d3392).
	namespace npc_scripts
	{
		struct Window
		{
			std::string script;
			// YYYYMMDD, both days included.
			int32_t start;
			int32_t end;
		};

		// The newest window open on `today` (FUN_006dedf7), or "" for none.
		inline std::string active(const std::vector<Window>& windows, int32_t today)
		{
			for (auto it = windows.rbegin(); it != windows.rend(); ++it)
				if (it->start <= today && today <= it->end)
					return it->script;

			return {};
		}

		// The windows 0x107 added, by NPC id. Like v83's template cache they
		// last the whole session and only grow.
		inline std::map<int32_t, std::vector<Window>>& received()
		{
			static std::map<int32_t, std::vector<Window>> windows;
			return windows;
		}
	}
}
