#pragma once

#include <cstdint>
#include <string>

// Catch items (227xxxx), thrown at a mob to catch it, as v83 does it (report 227).
namespace ms::catch_item
{
	// What a catch item says when no mob it catches is within its area, by its `bridleMsgType`; nothing for
	// the Fish Net's 4, or a 3 without a `nomobMsg` (FUN_00a09bdf, 0xa09eb9).
	inline std::string no_mob_notice(int32_t msg_type, const std::string &nomob_msg)
	{
		switch (msg_type)
		{
			case 0:
				return "Unable to find a tamable monster in this area.";
			case 1:
				return "Unable to capture the monster; the ghost is too far.";
			case 2:
				return "Unable to capture the monster; the monster is too far.";
			case 3:
				return nomob_msg;
			default:
				return "";
		}
	}

	// What BRIDLE_MOB_CATCH_FAIL says for its `result`: the mob is too strong, or the item, whose
	// `delayMsg` is given, was used too recently (FUN_00a0800e).
	inline std::string fail_notice(int8_t result, const std::string &delay_msg)
	{
		switch (result)
		{
			case 0:
				return "Unable to capture the monster; the monster is too strong.";
			case 1:
				return delay_msg.empty() ? "Element Rock cannot be used right after usage." : delay_msg;
			default:
				return "";
		}
	}

	// What a catch item says when there is no room for the item `create` it would make. v83 starts the
	// equip line with a tab, which HC's fonts can't draw.
	inline std::string no_room_notice(int32_t create)
	{
		switch (create / 1000000)
		{
			case 1:
				return "You do not have enough slots in your item inventory.";
			case 2:
				return "Use item inventory is full.";
			case 3:
				return "Setup item inventory is full.";
			default:
				return "Etc item inventory is full.";
		}
	}
}
