#include "Family.h"

#include <algorithm>
#include <unordered_map>
#include <utility>

namespace ms
{
	bool FamilyPrivilege::needs_target() const
	{
		return type == 0 || type == 1;
	}

	int32_t FamilyInfo::used_today(int32_t index) const
	{
		auto iter = used.find(index);

		return iter == used.end() ? 0 : iter->second;
	}

	FamilyChart::FamilyChart()
	{
		slots.fill(-1);
	}

	const FamilyNode* FamilyChart::at(size_t slot) const
	{
		if (slot >= SLOTS || slots[slot] < 0)
			return nullptr;

		return &nodes[slots[slot]];
	}

	int32_t FamilyChart::count(int32_t key) const
	{
		auto iter = counts.find(key);

		return iter == counts.end() ? 0 : iter->second;
	}

	bool FamilyChart::can_add_junior() const
	{
		const FamilyNode* owner = at(OWNER);

		return mine && owner && owner->juniors.size() < max_juniors;
	}

	bool FamilyChart::passed_senior(size_t slot) const
	{
		if (!mine || slot < 5 || slot >= SLOTS)
			return false;

		const FamilyNode* node = at(slot);

		if (!node || node->senior < 0)
			return false;

		return node->level > nodes[node->senior].level;
	}

	namespace
	{
		// StringPool 0x146d-0x1470 and the fee, 0x146c.
		constexpr const char* JUNIOR_BOTH = "%d Reputation Points for yourself and %d Reputation Points for your Senior will be deducted\\nand all lower Family members will be affected.\\nDo you really want to separate from the selected character?";
		constexpr const char* JUNIOR_ONE = "%d Reputation Points will be deducted from you, \\nand all lower Family members will be affected.\\nDo you really want to separate from the selected character?";
		constexpr const char* SENIOR_BOTH = "%d Reputation Points for the Senior and %d Reputation Points for the 2nd Senior will be deducted, and all lower Family members will be affected.\\nDo you really want to separate from the selected character?";
		constexpr const char* SENIOR_ONE = "%d Reputations for the Senior will be deducted, and\\nall lower Family members will be affected. Do you really want to separate from the selected character?";
		constexpr const char* FEE = "\\n\\nFee:%d Mesos";

		std::string format(std::string text, std::initializer_list<int32_t> values)
		{
			for (int32_t value : values)
			{
				size_t at = text.find("%d");

				if (at != std::string::npos)
					text.replace(at, 2, std::to_string(value));
			}

			return text;
		}
	}

	std::string FamilyParting::text() const
	{
		std::string result = second_rep > 0
			? format(junior ? JUNIOR_BOTH : SENIOR_BOTH, { rep, second_rep })
			: format(junior ? JUNIOR_ONE : SENIOR_ONE, { rep });

		if (fee > 0)
			result += format(FEE, { fee });

		return result;
	}

	FamilyParting FamilyChart::parting(const FamilyNode& other, bool junior) const
	{
		const FamilyNode& owner = *at(OWNER);
		const FamilyNode& senior = junior ? owner : other;
		const FamilyNode& below = junior ? other : owner;

		// The junior's level sets the Rep, the levels between the fee.
		int32_t level = below.level;
		int32_t rep = (level / 20 + 10) * level;
		int32_t gap = std::max(0, senior.level - below.level);

		FamilyParting result;
		result.id = other.id;
		result.name = other.name;
		result.junior = junior;
		result.rep = 2 * rep;
		result.second_rep = senior.senior >= 0 ? rep : 0;
		result.fee = (gap + 2500) * gap;

		return result;
	}

	std::optional<FamilyParting> FamilyChart::parting_at(size_t slot) const
	{
		const FamilyNode* owner = at(OWNER);
		const FamilyNode* other = at(slot);

		if (!mine || !owner || !other)
			return std::nullopt;

		if (owner->senior >= 0 && nodes[owner->senior].id == other->id)
			return parting(*other, false);

		for (int32_t junior : owner->juniors)
			if (nodes[junior].id == other->id)
				return parting(*other, true);

		return std::nullopt;
	}

	std::optional<FamilyParting> FamilyChart::parting_named(const std::string& name) const
	{
		const FamilyNode* owner = at(OWNER);

		if (!mine || !owner)
			return std::nullopt;

		if (owner->senior >= 0 && nodes[owner->senior].name == name)
			return parting(nodes[owner->senior], false);

		for (int32_t junior : owner->juniors)
			if (nodes[junior].name == name)
				return parting(nodes[junior], true);

		return std::nullopt;
	}

	void FamilyChart::arrange(const FamilyNode& self)
	{
		slots.fill(-1);

		int32_t owner = -1;
		std::unordered_map<int32_t, int32_t> by_id;

		// The leader is never put in the map, so nobody hangs from it.
		for (int32_t i = 1; i < static_cast<int32_t>(nodes.size()); i++)
		{
			FamilyNode& node = nodes[i];

			if (node.id == viewed)
			{
				owner = i;
				node.online = false;
			}

			auto senior = by_id.find(node.senior_id);

			if (senior != by_id.end())
			{
				node.senior = senior->second;
				nodes[senior->second].juniors.push_back(i);
			}

			by_id[node.id] = i;
		}

		if (nodes.size() > 2)
			slots[LEADER] = 0;

		if (owner < 0 && mine)
		{
			FamilyNode local;
			local.id = self.id;
			local.job = self.job;
			local.level = self.level;
			local.online = true;
			local.name = self.name;

			owner = static_cast<int32_t>(nodes.size());
			nodes.push_back(local);
		}

		if (owner < 0)
			return;

		slots[OWNER] = owner;

		auto junior = [&](int32_t node, size_t index)
		{
			if (node < 0 || index >= nodes[node].juniors.size())
				return -1;

			return nodes[node].juniors[index];
		};

		int32_t senior = nodes[owner].senior;

		if (senior >= 0)
		{
			slots[SENIOR] = senior;
			slots[1] = nodes[senior].senior;
			slots[4] = junior(senior, 1);
		}

		for (size_t i = 0; i < 2; i++)
		{
			int32_t first = junior(owner, i);
			slots[5 + i] = first;
			slots[7 + 2 * i] = junior(first, 0);
			slots[8 + 2 * i] = junior(first, 1);
		}
	}

	bool Family::loaded() const
	{
		return has_info;
	}

	bool Family::exists() const
	{
		return has_info && !info.name.empty();
	}

	const FamilyInfo& Family::get_info() const
	{
		return info;
	}

	const std::vector<FamilyPrivilege>& Family::get_privileges() const
	{
		return privileges;
	}

	bool Family::is_leader(int32_t self) const
	{
		return has_info && info.leader_id == self;
	}

	bool Family::can_use(size_t index) const
	{
		if (!exists() || index >= privileges.size())
			return false;

		const FamilyPrivilege& privilege = privileges[index];

		return info.rep >= privilege.cost
			&& info.used_today(static_cast<int32_t>(index)) < privilege.limit;
	}

	void Family::set_info(FamilyInfo value)
	{
		info = std::move(value);
		has_info = true;
	}

	void Family::set_privileges(std::vector<FamilyPrivilege> value)
	{
		privileges = std::move(value);
	}

	void Family::clear()
	{
		has_info = false;
		info = {};
		privileges.clear();
	}
}
