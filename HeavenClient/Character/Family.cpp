#include "Family.h"

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
