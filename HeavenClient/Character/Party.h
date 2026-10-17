#pragma once

#include <cstdint>
#include <string>
#include <vector>

namespace ms
{
	struct PartyMember
	{
		int32_t id = 0;
		// Where the party list puts them, from 0 to 5; a slot stays empty
		// when its member leaves.
		size_t slot = 0;
		std::string name;
		int16_t job = 0;
		int16_t level = 0;
		// Zero-based channel, or negative while offline.
		int32_t channel = -2;
		// Their map, or 0 when they are on another channel.
		int32_t map_id = 0;
		// Last HP the server reported, while they share the player's map.
		int32_t hp = 0;
		int32_t maxhp = 0;

		bool online() const;
	};

	// The party the player belongs to, as the server last described it.
	class Party
	{
	public:
		bool exists() const;
		int32_t get_id() const;
		int32_t get_leader() const;
		bool is_leader(int32_t cid) const;

		// Members in joining order, the player included.
		const std::vector<PartyMember>& get_members() const;
		const PartyMember* find(int32_t cid) const;
		const PartyMember* find(const std::string& name) const;

		// Members other than the player who are on the player's map.
		std::vector<const PartyMember*> on_map(int32_t self, int32_t map_id) const;

		// Replace the member list. Keeps the HP already known for members
		// that stay.
		void update(int32_t id, int32_t leader, std::vector<PartyMember> members);
		void set_leader(int32_t cid);
		void set_hp(int32_t cid, int32_t hp, int32_t maxhp);
		void clear();

		// Whether HP bars show over members' heads (the HP MARK button).
		bool shows_hp_marks() const;
		void toggle_hp_marks();

	private:
		int32_t id = 0;
		int32_t leader = 0;
		std::vector<PartyMember> members;
		bool hp_marks = true;
	};
}
