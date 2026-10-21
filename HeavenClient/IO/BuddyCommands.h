#pragma once

#include <cstdint>
#include <string>

namespace ms
{
	// v83's buddy list requests, with the checks the client makes before
	// asking the server. Each refusal is an OK notice.
	namespace BuddyCommands
	{
		// FUN_00531021. The Edit Group dialog's adds refuse without a word.
		void add(const std::string& name, const std::string& group, bool notify = true);
		// Only someone who asked, or isn't listed (FUN_00531270).
		void accept(int32_t cid);
		// Removes a buddy or turns down a request (FUN_005311c1).
		void remove(int32_t cid);
		// Shows cid's request to be added (FUN_00522d22).
		void ask(const std::string& name, int32_t cid);
		// Asks again about each request still unanswered, unless one is
		// showing, as v83 does once a map has loaded (FUN_00a3fecf).
		void show_requests();
	}
}
