#pragma once

#include "Guild.h"

#include <array>
#include <cstdint>
#include <string>
#include <vector>

namespace ms
{
	// What the ALLIANCE struct carries (FUN_004e475a).
	struct AllianceData
	{
		int32_t id = 0;
		std::string name;
		// The titles of ranks 1 to 5.
		std::array<std::string, 5> titles;
		// Every guild in it, the player's own included.
		std::vector<int32_t> guild_ids;
		// How many guilds it may hold.
		int32_t capacity = 0;
		std::string notice;
	};

	// The player's guild alliance ("Guild Union"), as the server last
	// described it: the alliance, and the other guilds in it. The player's
	// own guild stays in their Guild, with its members' alliance ranks.
	class Alliance
	{
	public:
		bool exists() const;
		const AllianceData& get_data() const;
		// The other guilds, never the player's own (FUN_00a3acf5).
		const std::vector<GuildData>& get_guilds() const;
		const GuildData* find_guild(int32_t id) const;
		// A rank's title, or "" for none (FUN_00a3ab25).
		const std::string& title(int32_t rank) const;
		// The lowest rank with a title (FUN_00a3ae1f): 3 if rank 4 has none,
		// 4 if rank 5 has none, else 5. 0 for no alliance.
		int32_t lowest_rank() const;
		// Whether anyone in the alliance holds a rank; true for no guild or
		// no alliance (FUN_00a3ad86).
		bool rank_held(const Guild& own, int32_t rank) const;
		// Members who would hear alliance chat (FUN_00a3ab7e): those online
		// in the other guilds, then in the player's own, but the player.
		// None if the player's guild is in no alliance.
		std::vector<int32_t> online_ids(const Guild& own, int32_t self) const;

		// Takes the alliance, leaving the guilds alone (FUN_00a3aa5f).
		void set(AllianceData data);
		// Forgets the alliance (FUN_00a02561), or the guilds (FUN_00a16906).
		void clear();
		void clear_guilds();
		// A guild's GUILDDATA (FUN_00a3acf5): the player's own goes to their
		// Guild, any other replaces the guild of its id or joins the end.
		void store(Guild& own, GuildData guild);
		void erase(int32_t guild_id);

		// A member of another guild; nullptr if the guild or the member is
		// not there.
		const GuildMember* set_online(int32_t guild_id, int32_t cid, bool online);
		bool set_level_job(int32_t guild_id, int32_t cid, int32_t level, int32_t job);
		// A new master (mode 0x19): the old one becomes rank 2, wherever
		// either is.
		void set_master(Guild& own, int32_t old_cid, int32_t new_cid);
		// Mode 0x1b, which looks in the player's guild first.
		void set_rank(Guild& own, int32_t cid, int32_t rank);
		void set_titles(std::array<std::string, 5> titles);
		void set_notice(std::string notice);

	private:
		GuildMember* member(int32_t guild_id, int32_t cid);
		GuildMember* member(int32_t cid);

		AllianceData data;
		std::vector<GuildData> guilds;
	};
}
