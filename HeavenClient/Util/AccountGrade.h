#pragma once

#include <cstdint>

// The grade bytes of the login result, as v83 keeps them (CWvsContext +0x2030
// and +0x203c, report b156).
namespace ms
{
	struct AccountGrade
	{
		uint8_t grade = 0;
		uint8_t subgrade = 0;

		// A GM's account (FUN_004f38d3): its unknown '/' lines go out as
		// chat, and it can't trade or open a store.
		bool gm() const
		{
			return grade & 0x01;
		}

		// The admin account (FUN_004f38d3 and FUN_004f38b4), which can't drop
		// items or mesos either.
		bool admin() const
		{
			return (grade & 0x11) == 0x11;
		}

		// The sub-grade's 0x80, which tells the server of each map it enters
		// (ADMIN_COMMAND 0x11).
		bool alerts_admins() const
		{
			return subgrade & 0x80;
		}

		// Whether the chat bar reads the GM slash commands (0x52c9cd and
		// FUN_0052f000).
		bool slash_commands() const
		{
			return gm() || (subgrade & 0xE0);
		}

		// The GM rank (FUN_0052f031): 0 may do the most, 5 is a player's.
		int rank() const
		{
			if (grade & 0x20)
				return 0;

			if (grade & 0x01)
				return 1;

			if (subgrade & 0x20)
				return 2;

			if (subgrade & 0x40)
				return 3;

			return subgrade & 0x80 ? 4 : 5;
		}
	};

	namespace account_grade
	{
		// StringPool 0x94, a chat line.
		constexpr const char* NOT_FOR_ADMIN = "Not available with an Admin account.";
		// StringPool 0x371, a notice.
		constexpr const char* NO_ITEM_DROP = "The admin can't drop items.";
		// StringPool 0x307, a notice.
		constexpr const char* NO_MESO_DROP = "The Admin account can't throw away the money.";
	}
}
