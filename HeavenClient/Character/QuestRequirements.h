#pragma once

#include <cstdint>
#include <string>
#include <vector>

// The quest window's "who may start this" lines, as v83 words them (FUN_0087fdc9, report b004).
namespace ms::QuestRequirements
{
	// The level line from the start check's bounds, 0 for none; without a start check at all it is
	// "No limit". Both bounds are joined by two spaces, and an upper bound alone leads with one.
	std::string levels(bool checked, int32_t lvmin, int32_t lvmax);

	// The job line from the start check's job ids, in their order: whole classes by their name,
	// advancement ranges as "4th job advancement Available to Bowman", and the rest one name each.
	std::string jobs(const std::vector<int32_t>& ids);
}
