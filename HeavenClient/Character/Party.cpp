#include "Party.h"

#include <utility>

namespace ms
{
	bool PartyMember::online() const
	{
		return channel >= 0;
	}

	bool Party::exists() const
	{
		return id != 0;
	}

	int32_t Party::get_id() const
	{
		return id;
	}

	int32_t Party::get_leader() const
	{
		return leader;
	}

	bool Party::is_leader(int32_t cid) const
	{
		return exists() && leader == cid;
	}

	const std::vector<PartyMember>& Party::get_members() const
	{
		return members;
	}

	const PartyMember* Party::find(int32_t cid) const
	{
		for (auto& member : members)
			if (member.id == cid)
				return &member;

		return nullptr;
	}

	const PartyMember* Party::find(const std::string& name) const
	{
		for (auto& member : members)
			if (member.name == name)
				return &member;

		return nullptr;
	}

	std::vector<const PartyMember*> Party::on_map(int32_t self, int32_t map_id) const
	{
		std::vector<const PartyMember*> result;

		for (auto& member : members)
			if (member.id != self && member.online() && member.map_id == map_id)
				result.push_back(&member);

		return result;
	}

	void Party::update(int32_t new_id, int32_t new_leader, std::vector<PartyMember> new_members)
	{
		for (auto& member : new_members)
		{
			if (auto known = find(member.id))
			{
				member.hp = known->hp;
				member.maxhp = known->maxhp;
			}
		}

		id = new_id;
		leader = new_leader;
		members = std::move(new_members);
	}

	void Party::set_leader(int32_t cid)
	{
		leader = cid;
	}

	void Party::set_hp(int32_t cid, int32_t hp, int32_t maxhp)
	{
		for (auto& member : members)
		{
			if (member.id == cid)
			{
				member.hp = hp;
				member.maxhp = maxhp;
			}
		}
	}

	void Party::clear()
	{
		id = 0;
		leader = 0;
		members.clear();
	}

	bool Party::shows_hp_marks() const
	{
		return hp_marks;
	}

	void Party::toggle_hp_marks()
	{
		hp_marks = !hp_marks;
	}
}
