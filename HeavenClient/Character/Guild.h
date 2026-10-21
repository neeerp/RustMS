#pragma once

#include <array>
#include <cstdint>
#include <string>
#include <vector>

namespace ms
{
	// A guild's emblem: a mark over a background, each in one of 16 colours.
	struct GuildEmblem
	{
		uint16_t background = 0;
		uint8_t background_color = 0;
		uint16_t mark = 0;
		uint8_t mark_color = 0;

		// The client draws an emblem only if the background or the mark is
		// set.
		bool shown() const;
		bool operator==(const GuildEmblem& other) const;
	};

	// The GuildMark.img/Mark folder of a mark, by its thousands.
	const char* guild_mark_category(uint16_t mark);

	struct GuildMember
	{
		int32_t id = 0;
		std::string name;
		int32_t job = 0;
		int32_t level = 0;
		// 1 for the master, down to 5.
		int32_t rank = 0;
		bool online = false;

		bool operator==(const GuildMember& other) const;
	};

	// What GUILDDATA carries.
	struct GuildData
	{
		int32_t id = 0;
		std::string name;
		// The titles of ranks 1 to 5.
		std::array<std::string, 5> titles;
		// The master first.
		std::vector<GuildMember> members;
		int32_t capacity = 0;
		GuildEmblem emblem;
		std::string notice;
		int32_t points = 0;
		int32_t alliance_id = 0;

		bool operator==(const GuildData& other) const;
	};

	// The player's guild, as the server last described it.
	class Guild
	{
	public:
		// The guild name prompt's bounds.
		static constexpr size_t MIN_NAME = 4;
		static constexpr size_t MAX_NAME = 12;
		// The rank panel's title prompt takes 10 letters.
		static constexpr size_t MAX_TITLE = 10;

		enum class Name
		{
			OK,
			TOO_SHORT,
			NOT_ALLOWED
		};

		// Checks a new guild's name as the client does (FUN_0079fc74), but
		// for the forbidden words: 4 to 12 letters or digits, with fewer
		// than four of 'I' and 'l'.
		static Name check_name(const std::string& name);

		enum class Title
		{
			OK,
			TOO_SHORT,
			NOT_ALLOWED,
			// Rank 4 cannot lose its title while rank 5 has one.
			LOWER_RANK,
			// Nor can a rank anyone holds.
			HELD
		};

		// The fewest letters a rank's title takes: ranks 1 to 3 need a
		// title, 4 and 5 may have none.
		static size_t min_title(int32_t rank);
		// Checks a title typed for a rank against the rank panel's working
		// titles as the panel's Modify button does (FUN_00916e8d), but for
		// the forbidden words: a title is checked as a guild name.
		Title check_title(const std::array<std::string, 5>& working, int32_t rank, const std::string& title) const;
		// Stores a checked title in the working titles. Rank 4 becomes
		// "Member" if rank 5 gets a title and 4 has none.
		static void set_title(std::array<std::string, 5>& working, int32_t rank, std::string title);
		// Whether saving the working titles would change any, ignoring
		// case (FUN_00917127).
		bool titles_differ(const std::array<std::string, 5>& working) const;

		bool exists() const;
		const GuildData& get_data() const;
		const GuildMember* find(int32_t cid) const;
		// A member by name, matching case (FUN_00a39282).
		const GuildMember* find_by_name(const std::string& name) const;
		// The client takes the first member for the master.
		bool is_master(int32_t cid) const;
		// A member's rank, or 0 for someone outside the guild.
		int32_t rank(int32_t cid) const;
		bool full() const;
		// A rank's title, or "" for none.
		const std::string& title(int32_t rank) const;
		// The lowest rank with a title, which members join at
		// (FUN_00a3930b): 3 if rank 4 has none, 4 if rank 5 has none, else
		// 5. 0 for no guild.
		int32_t lowest_rank() const;
		// Whether any member holds a rank; true for no guild
		// (FUN_00a39399).
		bool rank_held(int32_t rank) const;
		size_t online_count() const;
		// Members who would hear guild chat: those online but the player.
		std::vector<int32_t> online_ids(int32_t self) const;

		// Whether a member of rank `self` may expel one of rank `picked`,
		// 0 for none, by the guild window's rule (FUN_0090f17c): the master
		// expels anyone else, a Jr. Master only those ranked below.
		static bool may_kick(int32_t self, int32_t picked);
		// Whether a member of rank `self` may move one of rank `picked` up
		// or down a rank, by the same rule: the master raises anyone below
		// Jr. Master and lowers anyone else, a Jr. Master raises those below
		// Member and lowers those below Jr. Master; no one is lowered from
		// the lowest rank.
		static bool may_raise(int32_t self, int32_t picked);
		bool may_lower(int32_t self, int32_t picked) const;

		void update(GuildData data);
		void clear();
		// Someone joined, or left or was expelled.
		void add(GuildMember member);
		void remove(int32_t cid);
		// These return whether the member was found.
		bool set_rank(int32_t cid, int32_t rank);
		bool set_online(int32_t cid, bool online);
		bool set_level_job(int32_t cid, int32_t level, int32_t job);
		void set_titles(std::array<std::string, 5> titles);
		void set_notice(std::string notice);

	private:
		GuildMember* member(int32_t cid);

		GuildData data;
	};
}
