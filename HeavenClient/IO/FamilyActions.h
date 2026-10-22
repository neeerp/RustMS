#pragma once

#include <string>

namespace ms
{
	// What the Family windows, the character info window, the avatar menu
	// and the Family key share (reports b042, b044, b045).
	namespace FamilyActions
	{
		// The Family key and BtFamily on the player's own info: opens the
		// Family window or closes it (ToggleUI 0x1b).
		void toggle_window();
		// Opens the Family Tree if it isn't open (OpenUI 0x1c) and asks for
		// `name`'s.
		void open_tree(const std::string& name);
		// The Add Junior prompt (0x8082fe).
		void prompt_junior();
		// Asks `name` to be a junior (0xa340f6): nothing before the first
		// INFO, and a refusal without asking for the player's own name.
		void add_junior(const std::string& name);
		// Whether the avatar menu has its Family row (0x9504ae): the
		// family is known and has room below the player.
		bool can_add_junior();
		// A family text in the chat log, as v83's type 0xc.
		void chat_line(const std::string& text);
	}
}
