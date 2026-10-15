#pragma once

#include <cstdint>
#include <vector>

namespace ms
{
	// What a v83 NPC does while idle (MapleLegends client: CNpc::Update
	// FUN_005eaf3e and its chooser FUN_005eb2b4). NPCs never walk. Every 3 to
	// 9 s, an NPC that is standing either plays one of its actions once,
	// perhaps saying one of that action's lines, or says one of its
	// info/speak lines.
	namespace npc_idle
	{
		// The pause before the next choice is PAUSE_MIN + rand() % PAUSE_SPREAD ms.
		constexpr int32_t PAUSE_MIN = 3000;
		constexpr int32_t PAUSE_SPREAD = 6000;

		// The chooser takes rand() % ROLL before each modulo below.
		constexpr int32_t ROLL = 50;

		struct Choice
		{
			// The action to play once, or -1 for none.
			int32_t action = -1;
			// The action's line or, with no action, the info/speak line to
			// say; -1 for none.
			int32_t line = -1;
		};

		// `action_lines` has the number of speak lines of each action, in the
		// template's order: every animation but stand and move. `specials` is
		// how many of them have `special`; the draw only reaches the first
		// (actions - specials). Drawing `shop`, the action named "shop" (or
		// -1), does nothing. `idle_lines` is the number of info/speak lines.
		// `roll` and `line_roll` are rand() % ROLL. Returns whether the NPC
		// does anything.
		inline bool choose(const std::vector<int32_t>& action_lines, int32_t specials, int32_t shop, int32_t idle_lines,
			int32_t roll, int32_t line_roll, Choice& choice)
		{
			int32_t actions = static_cast<int32_t>(action_lines.size()) - specials;

			if (actions < 0)
				actions = 0;

			int32_t total = actions + idle_lines;

			if (total <= 0)
				return false;

			int32_t index = roll % total;

			if (index < actions)
			{
				if (index == shop)
					return false;

				int32_t lines = action_lines[index];
				choice.action = index;
				choice.line = lines > 0 ? line_roll % lines : -1;
				return true;
			}

			choice.action = -1;
			choice.line = index - actions;
			return true;
		}
	}
}
