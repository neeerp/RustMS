#pragma once

#include <cstdint>
#include <string>
#include <vector>

namespace ms
{
	// A GM's slash command, as v83's chat bar reads it (FUN_0052c40c) before
	// any of the players' commands.
	struct GmCommand
	{
		enum class Kind
		{
			// Not a GM command, or not one this rank may use.
			NONE,
			// /h N: ADMIN_COMMAND 0x10, hidden when N isn't 0.
			HIDE,
			// /ch N: a change to channel `channel` (from 0), asked as the
			// player's own.
			CHANNEL,
			// /u: ADMIN_COMMAND 0x11 0, the users on the map.
			USERS,
			// /w NAME MSG: ADMIN_COMMAND 0x1E, a warning.
			WARN,
			// /ban NAME: ADMIN_COMMAND 0x03.
			BAN,
			// /block NAME DAYS REASON: a dialog, then ADMIN_COMMAND 0x04;
			// `text` is the reason's type in Etc/BlockReason.img.
			BLOCK,
			// /uclip: ADMIN_COMMAND 0x11 0x0C.
			UCLIP,
			// /alert*, /notice* and /slide*: ADMIN_BROADCAST, `type` to
			// `scope`.
			BROADCAST,
			// /slideax: 0x86.
			SLIDE_AX
		};

		// ADMIN_BROADCAST's scopes and types.
		static constexpr uint8_t WORLD = 0;
		static constexpr uint8_t CHANNEL_SCOPE = 1;
		static constexpr uint8_t MAP = 2;
		static constexpr uint8_t NOTICE = 0;
		static constexpr uint8_t ALERT = 1;
		static constexpr uint8_t SLIDE = 4;

		Kind kind = Kind::NONE;
		std::string name;
		std::string text;
		bool on = false;
		int32_t channel = 0;
		int32_t days = 0;
		uint8_t scope = 0;
		uint8_t type = 0;

		// The command `input` is for an account of v83's GM `rank`
		// (AccountGrade::rank). Each is matched as v83 matches it: a name
		// alone, or a name, a space and more.
		static GmCommand parse(const std::string& input, int rank);

		// A /block's reason: its number from 1 among Etc/BlockReason.img's
		// `types`, matched as typed, or 0 if none is (0x52d9ee).
		int32_t block_reason(const std::vector<std::string>& types) const;
	};
}
