#pragma once

#include <array>
#include <cstdint>
#include <map>
#include <optional>
#include <string>
#include <vector>

namespace ms
{
	// An entitlement the family window offers, as the server lists them
	// (FAMILY_PRIVILEGE_LIST).
	struct FamilyPrivilege
	{
		uint8_t type = 0;
		// The Rep it costs and how often it may be used a day.
		int32_t cost = 0;
		int32_t limit = 0;
		std::string name;
		std::string description;

		// Types 0 and 1 ask for a family member's name first.
		bool needs_target() const;
	};

	// The player's own numbers (FAMILY_INFO_RESULT).
	struct FamilyInfo
	{
		int32_t rep = 0;
		int32_t total_rep = 0;
		int32_t today = 0;
		uint16_t juniors = 0;
		uint16_t max_juniors = 0;
		int32_t leader_id = 0;
		// Empty outside a family.
		std::string name;
		std::string precepts;
		// Times used today, by the privilege's place in the list.
		std::map<int32_t, int32_t> used;

		int32_t used_today(int32_t index) const;
	};

	// One member of a Family Tree.
	struct FamilyNode
	{
		int32_t id = 0;
		int32_t senior_id = 0;
		uint16_t job = 0;
		uint8_t level = 0;
		bool online = false;
		int32_t rep = 0;
		int32_t total_rep = 0;
		int32_t senior_rep = 0;
		int32_t today = 0;
		int32_t map_id = 0;
		int32_t minutes = 0;
		std::string name;

		// Places in FamilyChart::nodes, -1 for none.
		int32_t senior = -1;
		std::vector<int32_t> juniors;
	};

	// Parting the owner of the player's own tree from their senior or one
	// of their juniors (BtBye, report b048).
	struct FamilyParting
	{
		int32_t id = 0;
		std::string name;
		// A junior (SEPARATE_FAMILY_BY_SENIOR) or the senior (BY_JUNIOR).
		bool junior = false;
		// The Rep the confirmation says each side loses, the second 0 when
		// there is no one above, and the fee in mesos.
		int32_t rep = 0;
		int32_t second_rep = 0;
		int32_t fee = 0;

		// The confirmation (0x80b385, 0x80b5d2).
		std::string text() const;
	};

	// A Family Tree (FAMILY_CHART_RESULT), laid out as the client does
	// (0x80a5f7): eleven boxes, four generations around the one viewed.
	struct FamilyChart
	{
		static constexpr size_t SLOTS = 11;
		static constexpr size_t LEADER = 0, SENIOR = 2, OWNER = 3;

		int32_t viewed = 0;
		// Whether the tree is the player's own.
		bool mine = false;
		std::vector<FamilyNode> nodes;
		// Places in `nodes` by box, -1 when empty.
		std::array<int32_t, SLOTS> slots;
		// -1: everyone in the family, 0: the seniors above the one viewed,
		// a junior's id: everyone below that junior.
		std::map<int32_t, int32_t> counts;
		std::map<int32_t, int32_t> used;
		uint16_t max_juniors = 0;

		FamilyChart();

		// The member in a box, or nullptr.
		const FamilyNode* at(size_t slot) const;
		int32_t count(int32_t key) const;
		// The Add Junior button: the player's own tree, with room below.
		bool can_add_junior() const;
		// Whether a member's name shows red: a junior of the player's tree
		// who passed their senior's level.
		bool passed_senior(size_t slot) const;
		// Parting from the member in a box, or the one named (senior
		// first), when they are the owner's senior or junior.
		std::optional<FamilyParting> parting_at(size_t slot) const;
		std::optional<FamilyParting> parting_named(const std::string& name) const;

		// Links the members as the client does: the first is the leader at
		// the top and nobody's senior, the one viewed is the owner, and the
		// others hang from their seniors. Without the owner in the packet,
		// the player's own tree gets one from `self`.
		void arrange(const FamilyNode& self);

	private:
		FamilyParting parting(const FamilyNode& other, bool junior) const;
	};

	// The player's family, as the server last described it.
	class Family
	{
	public:
		// The longest precepts the prompt takes.
		static constexpr size_t MAX_PRECEPTS = 200;

		// Every send but the two openers waits for the first INFO.
		bool loaded() const;
		bool exists() const;
		const FamilyInfo& get_info() const;
		const std::vector<FamilyPrivilege>& get_privileges() const;
		// Whether `self` may edit the precepts: the leader only.
		bool is_leader(int32_t self) const;
		// Whether the privilege at `index` may be used now.
		bool can_use(size_t index) const;

		void set_info(FamilyInfo info);
		void set_privileges(std::vector<FamilyPrivilege> privileges);
		void clear();

	private:
		bool has_info = false;
		FamilyInfo info;
		std::vector<FamilyPrivilege> privileges;
	};
}
